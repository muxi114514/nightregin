//! nrga 协议：只读 launcher\data\img 下的 webp，前端用 http://nrga.localhost/img/x.webp 取

use launcher_core::wiki::read_data_image;
use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{HeaderValue, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;

pub const SCHEME: &str = "nrga";

/// 读盘放到阻塞线程：一屏几十张图，别卡主线程
pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().trim_start_matches('/').to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = read_data_image(app.state::<AppState>().pack(), &path);
        responder.respond(image_response(bytes));
    });
}

pub fn image_response(bytes: Option<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(bytes) = bytes else {
        let mut missing = Response::new(Vec::new());
        *missing.status_mut() = StatusCode::NOT_FOUND;
        return missing;
    };
    let mut found = Response::new(bytes);
    let headers = found.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    // 更新后文件名不变、内容会变
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_response_ok() {
        let r = image_response(Some(vec![1, 2, 3]));
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers()["content-type"], "image/webp");
        assert_eq!(r.headers()["cache-control"], "no-cache");
        assert_eq!(r.body(), &vec![1, 2, 3]);
    }

    #[test]
    fn image_response_missing() {
        let r = image_response(None);
        assert_eq!(r.status(), 404);
        assert!(r.body().is_empty());
    }
}
