//! 上游平台适配：QQ 音乐与酷狗音乐。
//!
//! 两端都只用公开免登录接口：
//! - QQ：`u.y.qq.com/cgi-bin/musicu.fcg`（移动端搜索 + `GetPlayLyricInfo` 取加密 QRC）
//! - 酷狗：`mobilecdn.kugou.com` 搜索 + `lyrics.kugou.com` 取 KRC
//!
//! 所有 HTTP 细节都收敛在本模块，上游改版时只需改这里。

pub mod kugou;
pub mod qq;

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Client;

use crate::model::{AppError, SongCandidate, Source};

/// 默认 UA，按桌面 Chrome 伪装，两个平台都接受。
pub const DESKTOP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// 单次请求上限。上游偶尔挂住连接，必须设上限，否则界面会一直转圈。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent(DESKTOP_UA)
        .timeout(REQUEST_TIMEOUT)
        // 上游证书链偶有缺失，但带完整 CA 时不需要放宽校验；这里只关掉 HTTP/2 的
        // 提前知识探测，避免部分 CDN 对 h2 握手返回 400。
        .http1_only()
        .build()
        .expect("构造 HTTP 客户端失败")
});

pub fn client() -> &'static Client {
    &CLIENT
}

/// 把 `reqwest::Error` 翻成中文错误。
pub fn map_http_error(context: &str, error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::network(format!("{context}超时，请检查网络后重试"))
    } else if error.is_connect() {
        AppError::network(format!("{context}无法连接，请检查网络或代理设置"))
    } else {
        AppError::network(format!("{context}请求失败：{error}"))
    }
}

/// 按来源搜索歌曲。
pub async fn search(
    source: Option<Source>,
    keyword: &str,
    limit: usize,
) -> Result<Vec<SongCandidate>, AppError> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }

    match source {
        Some(Source::Qq) => qq::search(keyword, limit).await,
        Some(Source::Kugou) => kugou::search(keyword, limit).await,
        None => {
            // 全部来源并发；一侧失败不影响另一侧，只有两侧都失败才报错。
            let (qq_result, kugou_result) =
                tokio::join!(qq::search(keyword, limit), kugou::search(keyword, limit));
            match (qq_result, kugou_result) {
                (Ok(mut a), Ok(b)) => {
                    a.extend(b);
                    Ok(a)
                }
                (Ok(a), Err(_)) => Ok(a),
                (Err(_), Ok(b)) => Ok(b),
                (Err(qq_error), Err(kugou_error)) => Err(AppError::network(format!(
                    "QQ 音乐与酷狗都请求失败。\nQQ：{}\n酷狗：{}",
                    qq_error.message, kugou_error.message
                ))),
            }
        }
    }
}
