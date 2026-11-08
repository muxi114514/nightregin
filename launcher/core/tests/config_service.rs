//! 配置写入队列：300ms 防抖（同一种改动只留最新）、flush 立刻写完、失败留着重试、手改后刷新

mod common;

use std::fs;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use common::{FakePack, fake_pack};
use launcher_core::config::ConfigFile;
use launcher_core::config::game::{Field, SettingChange};
use launcher_core::config::queue::{DEBOUNCE, WriteQueue};
use launcher_core::config::service::{ConfigEvent, ConfigFlush, ConfigService};
use launcher_core::ini::IniDoc;
use launcher_core::nighter::read_nighter;
use launcher_core::paths::PackLayout;

const SHORT: Duration = Duration::from_millis(20);

fn pack(fp: &FakePack) -> PackLayout {
    PackLayout {
        root: fp.pack_root.clone(),
    }
}

fn level_on_disk(fp: &FakePack) -> u8 {
    read_nighter(&ConfigFile::Nighter.path(&pack(fp)))
        .unwrap()
        .level
}

fn set_readonly(path: &std::path::Path, on: bool) {
    let mut perm = fs::metadata(path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(on);
    fs::set_permissions(path, perm).unwrap();
}

/// 起后台写线程，事件进 channel
struct Worker {
    events: mpsc::Receiver<ConfigEvent>,
    service: Arc<ConfigService>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn start(service: Arc<ConfigService>) -> Worker {
        let (tx, events) = mpsc::channel();
        let tx = Mutex::new(tx);
        let bg = Arc::clone(&service);
        let handle = thread::spawn(move || {
            bg.run_worker(&|e| {
                let _ = tx.lock().unwrap().send(e);
            });
        });
        Worker {
            events,
            service,
            handle: Some(handle),
        }
    }

    fn next(&self) -> ConfigEvent {
        self.events
            .recv_timeout(Duration::from_secs(5))
            .expect("没等到写入事件")
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.service.shutdown();
        if let Some(h) = self.handle.take() {
            h.join().unwrap();
        }
    }
}

#[test]
fn debounce_is_300ms() {
    assert_eq!(DEBOUNCE, Duration::from_millis(300));
}

#[test]
fn queue_debounces_from_last_change() {
    let t0 = Instant::now();
    let mut q = WriteQueue::default();
    assert_eq!(q.next_wait(t0), None);
    q.push(SettingChange::DeepNightLevel(3), t0);
    assert_eq!(q.next_wait(t0), Some(DEBOUNCE));
    q.push(
        SettingChange::DeepNightLevel(4),
        t0 + Duration::from_millis(200),
    );
    assert_eq!(
        q.next_wait(t0 + Duration::from_millis(400)),
        Some(Duration::from_millis(100))
    );
    assert_eq!(
        q.next_wait(t0 + Duration::from_millis(500)),
        Some(Duration::ZERO)
    );
    assert_eq!(
        q.next_wait(t0 + Duration::from_millis(900)),
        Some(Duration::ZERO)
    );
}

#[test]
fn queue_keeps_newest_per_key() {
    let t0 = Instant::now();
    let mut q = WriteQueue::default();
    q.push(SettingChange::DeepNightLevel(3), t0);
    q.push(SettingChange::HealthScaling(90), t0);
    q.push(SettingChange::DeepNightLevel(4), t0);
    let taken = q.take();
    assert_eq!(taken.len(), 2);
    assert!(taken.contains(&SettingChange::DeepNightLevel(4)));
    assert!(taken.contains(&SettingChange::HealthScaling(90)));
    assert!(q.pending().is_empty());
    assert_eq!(q.next_wait(t0), None);
}

#[test]
fn put_back_does_not_clobber_newer() {
    let t0 = Instant::now();
    let mut q = WriteQueue::default();
    q.push(SettingChange::DeepNightLevel(3), t0);
    q.push(SettingChange::HealthScaling(90), t0);
    let failed = q.take();
    q.push(SettingChange::DeepNightLevel(5), t0);
    q.put_back(failed);
    let now: Vec<_> = q.pending().to_vec();
    assert_eq!(now.len(), 2);
    assert!(now.contains(&SettingChange::DeepNightLevel(5)));
    assert!(now.contains(&SettingChange::HealthScaling(90)));
}

#[test]
fn change_is_written_after_debounce() {
    let fp = fake_pack("pack");
    let worker = Worker::start(ConfigService::with_debounce(pack(&fp), SHORT));
    worker.service.change(SettingChange::DeepNightLevel(3));
    assert!(matches!(worker.next(), ConfigEvent::Applied { .. }));
    assert_eq!(level_on_disk(&fp), 3);
}

#[test]
fn rapid_changes_write_once() {
    let fp = fake_pack("pack");
    let worker = Worker::start(ConfigService::with_debounce(
        pack(&fp),
        Duration::from_millis(120),
    ));
    for level in [1, 2, 3, 4, 2] {
        worker.service.change(SettingChange::DeepNightLevel(level));
        thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(worker.next(), ConfigEvent::Applied { .. }));
    assert_eq!(level_on_disk(&fp), 2);
    assert!(
        worker
            .events
            .recv_timeout(Duration::from_millis(300))
            .is_err(),
        "只应写一次"
    );
}

#[test]
fn view_overlays_pending() {
    let fp = fake_pack("pack");
    let service = ConfigService::with_debounce(pack(&fp), Duration::from_secs(60));
    service.change(SettingChange::DeepNightLevel(2));
    assert_eq!(service.view().deep_night_level, Field::Ok { value: 2 });
    assert_eq!(level_on_disk(&fp), 5);
}

#[test]
fn flush_writes_immediately() {
    let fp = fake_pack("pack");
    let service = ConfigService::with_debounce(pack(&fp), Duration::from_secs(60));
    service.change(SettingChange::DeepNightLevel(2));
    service.flush().unwrap();
    assert_eq!(level_on_disk(&fp), 2);
    assert_eq!(service.view().deep_night_level, Field::Ok { value: 2 });
}

#[test]
fn flush_with_nothing_pending_is_ok() {
    let fp = fake_pack("pack");
    ConfigService::with_debounce(pack(&fp), SHORT)
        .flush()
        .unwrap();
}

#[test]
fn failed_write_keeps_pending_and_flush_retries() {
    let fp = fake_pack("pack");
    let path = ConfigFile::Nighter.path(&pack(&fp));
    let service = ConfigService::with_debounce(pack(&fp), Duration::from_secs(60));
    service.change(SettingChange::DeepNightLevel(2));
    set_readonly(&path, true);
    assert!(service.flush().is_err());
    assert_eq!(service.view().deep_night_level, Field::Ok { value: 2 });
    set_readonly(&path, false);
    service.flush().unwrap();
    assert_eq!(level_on_disk(&fp), 2);
}

#[test]
fn worker_reports_write_failure() {
    let fp = fake_pack("pack");
    let path = ConfigFile::Nighter.path(&pack(&fp));
    set_readonly(&path, true);
    let worker = Worker::start(ConfigService::with_debounce(pack(&fp), SHORT));
    worker.service.change(SettingChange::DeepNightLevel(2));
    assert!(matches!(worker.next(), ConfigEvent::WriteFailed(_)));
    set_readonly(&path, false);
}

#[test]
fn worker_refreshes_view_after_external_edit() {
    let fp = fake_pack("pack");
    let layout = pack(&fp);
    let worker = Worker::start(ConfigService::with_debounce(layout.clone(), SHORT));
    // 界面打开时读过一次
    worker.service.view();
    let nighter = ConfigFile::Nighter.path(&layout);
    let text = fs::read_to_string(&nighter).unwrap();
    fs::write(
        &nighter,
        text.replace(
            "\"forceUnlockDeepNight\": true",
            "\"forceUnlockDeepNight\": false",
        ),
    )
    .unwrap();

    worker.service.change(SettingChange::DeepNightLevel(3));
    assert!(matches!(
        worker.next(),
        ConfigEvent::Applied {
            external_change: true
        }
    ));
    let view = worker.service.view();
    assert_eq!(view.unlock_deep_night, Field::Ok { value: false });
    assert_eq!(view.deep_night_level, Field::Ok { value: 3 });
}

#[test]
fn align_suffix_flushes_first_and_writes_now() {
    let fp = fake_pack("pack");
    let layout = pack(&fp);
    let service = ConfigService::with_debounce(layout.clone(), Duration::from_secs(60));
    service.change(SettingChange::DeepNightLevel(2));
    assert!(service.align_suffix().unwrap().wrote);
    assert_eq!(level_on_disk(&fp), 2);
    let doc = IniDoc::parse(&fs::read(ConfigFile::ChanceSetter.path(&layout)).unwrap());
    assert_eq!(doc.get("save", "autoExtension").as_deref(), Some(".co2"));
}

#[test]
fn fix_everdark_goes_through_service() {
    let fp = fake_pack("pack");
    let layout = pack(&fp);
    let service = ConfigService::with_debounce(layout.clone(), Duration::from_secs(60));
    // 手改过 nighter 的永夜名单
    let nighter = ConfigFile::Nighter.path(&layout);
    let mut doc = launcher_core::nighter::NighterDoc::parse(&fs::read(&nighter).unwrap()).unwrap();
    doc.set_super_lords(&[1]).unwrap();
    fs::write(&nighter, doc.to_bytes()).unwrap();
    assert!(service.fix_everdark().unwrap().wrote);
    assert_eq!(read_nighter(&nighter).unwrap().super_lords.len(), 7);
}

/// ConfigFlush 是 PackEnv 用的接缝
#[test]
fn service_is_a_flusher() {
    let fp = fake_pack("pack");
    let service = ConfigService::with_debounce(pack(&fp), Duration::from_secs(60));
    let flusher: Arc<dyn ConfigFlush> = service.clone();
    service.change(SettingChange::HealthScaling(90));
    flusher.flush().unwrap();
    let doc = IniDoc::parse(&fs::read(ConfigFile::Nrsc.path(&pack(&fp))).unwrap());
    assert_eq!(doc.get("SCALING", "health_scaling").as_deref(), Some("90"));
}

/// 文件缺失、内容不对不是暂时性失败：报一次就丢掉，不能一直卡住开始游戏
#[test]
fn permanent_failure_is_not_retried() {
    let fp = fake_pack("pack");
    let layout = pack(&fp);
    let service = ConfigService::with_debounce(layout.clone(), Duration::from_secs(60));
    fs::remove_file(ConfigFile::ChanceSetter.path(&layout)).unwrap();
    service.change(SettingChange::Lords(vec![1]));
    assert!(service.flush().is_err());
    service.flush().unwrap();
}

#[test]
fn changed_on_disk_after_manual_edit() {
    let fp = fake_pack("pack");
    let layout = pack(&fp);
    let service = ConfigService::with_debounce(layout.clone(), Duration::from_secs(60));
    service.view();
    assert!(!service.changed_on_disk());
    let nighter = ConfigFile::Nighter.path(&layout);
    let text = fs::read_to_string(&nighter).unwrap();
    fs::write(&nighter, text.replace("\"level\": 5", "\"level\": 3")).unwrap();
    assert!(service.changed_on_disk());
    assert_eq!(service.view().deep_night_level, Field::Ok { value: 3 });
    assert!(!service.changed_on_disk());
    fs::remove_file(&nighter).unwrap();
    assert!(service.changed_on_disk(), "文件被删也算变了");
}
