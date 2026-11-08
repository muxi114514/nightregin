// 真实接口：命令名与事件名对应 app\src\commands.rs；参数用 camelCase，Tauri 自动转成 snake_case
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AboutView, Api, BackupNow, BackupRow, ChooseDir, FirstRunPlan, GameSettings, HomeState, LaunchView, OptionsView,
  ReportPreview, SavesView, VanillaCopy, WikiOdds, WikiView,
} from './types';

const EVENT = {
  launchView: 'launch-view',
  homeState: 'home-state',
  visibility: 'window-visibility',
  closeRequested: 'close-requested',
  gameSettings: 'game-settings',
  configWriteFailed: 'config-write-failed',
  openReport: 'open-report',
} as const;

/** 命令的拒绝原因是 Rust 给的中文字符串，统一包成 Error */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw e instanceof Error ? e : new Error(String(e));
  }
}

/** listen 是异步注册的，注册完成才 resolve；注册失败只记日志，界面其余部分照常 */
async function subscribe<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  try {
    return await listen<T>(event, (e) => {
      try {
        cb(e.payload);
      } catch (err) {
        console.error(`处理事件 ${event} 出错`, err);
      }
    });
  } catch (err) {
    console.error(`订阅事件 ${event} 失败`, err);
    return () => {};
  }
}

export function createTauriApi(): Api {
  return {
    uiReady: () => call<void>('ui_ready'),
    getHome: () => call<HomeState>('get_home'),
    startGame: async ({ skipBackup, ignoreVersion }) => {
      await call<null>('start_game', { skipBackup, ignoreVersion });
    },
    cancelLaunch: () => call<boolean>('cancel_launch'),
    endProcesses: () => call<void>('end_processes'),
    focusGame: () => call<boolean>('focus_game'),
    chooseGameDir: () => call<ChooseDir>('choose_game_dir'),
    getLaunchView: () => call<LaunchView>('get_launch_view'),
    getGameSettings: () => call<GameSettings>('get_game_settings'),
    setGameSetting: (change) => call<void>('set_game_setting', { change }),
    restoreConfigDefault: (file) => call<GameSettings>('restore_config_default', { file }),
    fixEverdark: () => call<void>('fix_everdark'),
    alignSuffix: () => call<void>('align_suffix'),
    getSaves: (account) => call<SavesView>('get_saves', { account }),
    backupNow: (account, force) => call<BackupNow>('backup_now', { account, force }),
    restoreBackup: (account, name) => call<BackupRow | null>('restore_backup', { account, name }),
    openSaveFolder: (account) => call<void>('open_save_folder', { account }),
    setSaveAccount: (account) => call<void>('set_save_account', { account }),
    copyFromVanilla: (account) => call<VanillaCopy>('copy_from_vanilla', { account }),
    getOptions: () => call<OptionsView>('get_options'),
    setOption: (change) => call<void>('set_option', { change }),
    getAbout: () => call<AboutView>('get_about'),
    createShortcut: () => call<string>('create_shortcut'),
    ackOpenReport: () => call<void>('ack_open_report'),
    previewReport: () => call<ReportPreview>('preview_report'),
    createReport: () => call<string>('create_report'),
    revealPath: (path) => call<void>('reveal_path', { path }),
    getFirstRun: () => call<FirstRunPlan>('get_first_run'),
    finishFirstRun: (account) => call<void>('finish_first_run', { account: account ?? null }),
    getWiki: () => call<WikiView>('get_wiki'),
    getWikiOdds: () => call<WikiOdds>('get_wiki_odds'),
    // 只跑在 Windows；convertFileSrc 会把路径里的 / 编码掉
    wikiImageUrl: (rel) => `http://nrga.localhost/${rel}`,
    minimizeWindow: () => call<void>('minimize_window'),
    closeApp: () => call<void>('close_app'),
    onLaunchView: (cb) => subscribe<LaunchView>(EVENT.launchView, cb),
    onHomeState: (cb) => subscribe<HomeState>(EVENT.homeState, cb),
    onVisibility: (cb) =>
      subscribe<{ visible?: unknown }>(EVENT.visibility, (p) => {
        if (typeof p?.visible === 'boolean') cb(p.visible);
        else console.warn('window-visibility 负载不对', p);
      }),
    onCloseRequested: (cb) => subscribe<unknown>(EVENT.closeRequested, () => cb()),
    onGameSettings: (cb) => subscribe<GameSettings>(EVENT.gameSettings, cb),
    onConfigWriteFailed: (cb) => subscribe<string>(EVENT.configWriteFailed, cb),
    onOpenReport: (cb) => subscribe<unknown>(EVENT.openReport, () => cb()),
  };
}
