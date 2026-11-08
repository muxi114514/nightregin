//! 文件操作遇到暂时性失败（杀毒软件、索引程序短暂占用文件）时有限次重试

use std::io;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION;

/// 最多试几次
const ATTEMPTS: u32 = 3;
/// 两次之间的间隔
const INTERVAL: Duration = Duration::from_millis(50);

/// 只对「拒绝访问」和「共享冲突」重试，其余错误立即返回；最后一次的错误原样返回
pub(crate) fn transient<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 1;
    loop {
        match op() {
            Err(e) if attempt < ATTEMPTS && is_transient(&e) => {
                attempt += 1;
                thread::sleep(INTERVAL);
            }
            result => return result,
        }
    }
}

/// 共享冲突在 std 里没有专门的 ErrorKind，只能按系统错误码认
fn is_transient(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::PermissionDenied
        || e.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::transient;

    fn os_error(code: i32) -> io::Error {
        io::Error::from_raw_os_error(code)
    }

    #[test]
    fn retry_only_transient_errors() {
        // 共享冲突两次后成功：一共试 3 次
        let mut calls = 0;
        let result = transient(|| {
            calls += 1;
            if calls < 3 {
                Err(os_error(32))
            } else {
                Ok(calls)
            }
        });
        assert_eq!(result.unwrap(), 3);

        // 一直拒绝访问：试满 3 次后把最后的错误交出来
        let mut calls = 0;
        let err = transient(|| -> io::Result<()> {
            calls += 1;
            Err(os_error(5))
        })
        .unwrap_err();
        assert_eq!((calls, err.kind()), (3, io::ErrorKind::PermissionDenied));

        // 找不到文件不是暂时性的：只试 1 次
        let mut calls = 0;
        let err = transient(|| -> io::Result<()> {
            calls += 1;
            Err(os_error(2))
        })
        .unwrap_err();
        assert_eq!((calls, err.kind()), (1, io::ErrorKind::NotFound));
    }
}
