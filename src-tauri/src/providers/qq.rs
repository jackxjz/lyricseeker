//! QQ 音乐：搜索 + 取逐字歌词（QRC）。
//!
//! 端点与参数来自对 `u.y.qq.com` 统一接口的实测：
//! - 搜索：`music.search.SearchCgiService / DoSearchForQQMusicMobile`
//! - 取词：`music.musichallSong.PlayLyricInfo / GetPlayLyricInfo`（`crypt:1` 返回加密 QRC）
//!
//! 取回的密文经 `lyrics_crypto` 解出 XML，再从中取 `Lyric_1@LyricContent` 正文。

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use lyrics_crypto::decrypter::qrc::decrypter::decrypt_lyrics as decrypt_qrc;
use lyrics_parsers::xml_utils;

use crate::model::{AppError, SongCandidate, Source};
use crate::providers::{client, map_http_error};

const MUSICU_URL: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";

/// QQ 搜索的内层风控码：请求过频时返回这个值。
const BLOCKED_CODE: i32 = 2001;

/// QQ 接口要求 Referer 为站点域名，否则取词会被拒。
const QQ_HEADERS: &[(&str, &str)] = &[
    ("Referer", "https://y.qq.com/"),
    ("Origin", "https://y.qq.com"),
    ("Content-Type", "application/json"),
];

// ---------------------------------------------------------------------------
// 请求体
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct Comm {
    ct: i32,
    cv: String,
    v: String,
    os_ver: String,
    phonetype: String,
    rom: String,
    #[serde(rename = "tmeAppID")]
    tme_app_id: String,
    nettype: String,
    udid: String,
    uid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    userip: Option<String>,
}

impl Comm {
    fn anonymous() -> Self {
        Self {
            ct: 11,
            cv: "1003006".to_string(),
            v: "1003006".to_string(),
            os_ver: "15".to_string(),
            phonetype: "24122RKC7C".to_string(),
            rom: "Redmi/miro/miro:15/AE3A.240806.005/OS2.0.105.0.VOMCNXM:user/release-keys"
                .to_string(),
            tme_app_id: "qqmusiclight".to_string(),
            nettype: "NETWORK_WIFI".to_string(),
            udid: "0".to_string(),
            uid: "0".to_string(),
            sid: None,
            userip: None,
        }
    }

    fn with_session(uid: String, sid: String, userip: String) -> Self {
        Self {
            uid,
            sid: Some(sid),
            userip: Some(userip),
            ..Self::anonymous()
        }
    }
}

#[derive(Serialize)]
struct SearchParam {
    search_id: String,
    remoteplace: String,
    query: String,
    search_type: i32,
    num_per_page: i32,
    page_num: i32,
    highlight: i32,
    nqc_flag: i32,
    page_id: i32,
    grp: i32,
}

#[derive(Serialize)]
struct RequestBody {
    method: String,
    module: String,
    param: SearchParam,
}

#[derive(Serialize)]
struct MusicuBody {
    comm: Comm,
    request: RequestBody,
}

#[derive(Serialize)]
struct LyricsParam {
    #[serde(rename = "songMID")]
    song_mid: String,
    #[serde(rename = "songID")]
    song_id: i64,
    #[serde(rename = "songName")]
    song_name: String,
    #[serde(rename = "singerName")]
    singer_name: String,
    #[serde(rename = "albumName")]
    album_name: String,
    interval: i32,
    #[serde(rename = "lrc_t")]
    lrc_t: i32,
    #[serde(rename = "qrc_t")]
    qrc_t: i32,
    #[serde(rename = "trans_t")]
    trans_t: i32,
    #[serde(rename = "roma_t")]
    roma_t: i32,
    crypt: i32,
    ct: i32,
    cv: i32,
    qrc: i32,
    roma: i32,
    trans: i32,
    #[serde(rename = "type")]
    lyric_type: i32,
}

#[derive(Serialize)]
struct LyricsBody {
    comm: Comm,
    request: LyricsRequestBody,
}

#[derive(Serialize)]
struct LyricsRequestBody {
    method: String,
    module: String,
    param: LyricsParam,
}

// ---------------------------------------------------------------------------
// 响应体
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MusicuResponse {
    code: Option<i32>,
    request: Option<SearchReq>,
}

/// 桌面端接口的响应把业务对象放在 `req` 里（与移动端的 `request` 不同）。
#[derive(Debug, Deserialize)]
struct DesktopResponse {
    code: Option<i32>,
    req: Option<SearchReq>,
}

#[derive(Debug, Deserialize)]
struct SearchReq {
    code: Option<i32>,
    data: Option<SearchData>,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    body: Option<SearchBody>,
}

#[derive(Debug, Deserialize)]
struct SearchBody {
    /// 移动端接口用 `item_song` 包装歌曲。
    item_song: Option<Vec<SongItem>>,
    /// 桌面端接口用 `song.list` 包装歌曲。
    song: Option<DesktopSongs>,
}

#[derive(Debug, Deserialize)]
struct DesktopSongs {
    list: Option<Vec<DesktopSongItem>>,
}

/// 桌面端搜索返回的歌曲结构（字段与移动端不同）。
#[derive(Debug, Deserialize)]
struct DesktopSongItem {
    #[serde(rename = "id")]
    numeric_id: Option<i64>,
    mid: String,
    title: String,
    singer: Option<Vec<Singer>>,
    /// 歌手名可能是字符串而非数组，统一按可选字符串兜底。
    #[serde(rename = "singer_name")]
    singer_name: Option<String>,
    album: Option<Album>,
    interval: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct SongItem {
    #[serde(rename = "id")]
    numeric_id: Option<i64>,
    mid: String,
    title: String,
    singer: Option<Vec<Singer>>,
    album: Option<Album>,
    /// 秒
    interval: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Singer {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Album {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LyricsResponse {
    code: Option<i32>,
    request: Option<LyricsReq>,
}

#[derive(Debug, Deserialize)]
struct LyricsReq {
    data: Option<LyricsDataInner>,
}

#[derive(Debug, Deserialize)]
struct LyricsDataInner {
    lyric: Option<String>,
    trans: Option<String>,
    roma: Option<String>,
    #[serde(rename = "lrc_t")]
    lrc_t: Option<i32>,
    #[serde(rename = "qrc_t")]
    qrc_t: Option<i32>,
    #[serde(rename = "trans_t")]
    trans_t: Option<i32>,
    #[serde(rename = "roma_t")]
    roma_t: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    code: Option<i32>,
    request: Option<SessionReq>,
}

#[derive(Debug, Deserialize)]
struct SessionReq {
    data: Option<SessionData>,
}

#[derive(Debug, Deserialize)]
struct SessionData {
    session: Option<SessionInfo>,
}

#[derive(Debug, Deserialize)]
struct SessionInfo {
    uid: Option<serde_json::Value>,
    sid: Option<String>,
    userip: Option<String>,
}

// ---------------------------------------------------------------------------
// 搜索
// ---------------------------------------------------------------------------

/// 匿名 session 缓存；拿不到就退化成匿名身份，不影响搜索与取词。
static SESSION: OnceCell<Option<(String, String, String)>> = OnceCell::const_new();

async fn comm() -> Comm {
    match SESSION.get_or_init(fetch_session).await {
        Some((uid, sid, userip)) => Comm::with_session(uid.clone(), sid.clone(), userip.clone()),
        None => Comm::anonymous(),
    }
}

async fn fetch_session() -> Option<(String, String, String)> {
    let body = serde_json::json!({
        "comm": Comm::anonymous(),
        "request": {
            "method": "GetSession",
            "module": "music.getSession.session",
            "param": { "caller": 0, "uid": "0", "vkey": 0 }
        }
    });

    let response = client()
        .post(MUSICU_URL)
        .headers(headers(QQ_HEADERS))
        .json(&body)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let parsed: SessionResponse = response.json().await.ok()?;
    let session = parsed.request?.data?.session?;
    let uid = match session.uid {
        Some(serde_json::Value::Number(number)) => number.to_string(),
        Some(serde_json::Value::String(text)) => text,
        _ => "0".to_string(),
    };
    Some((
        uid,
        session.sid.unwrap_or_default(),
        session.userip.unwrap_or_default(),
    ))
}

fn headers(extra: &[(&'static str, &'static str)]) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (name, value) in extra {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(name.as_bytes()),
            reqwest::header::HeaderValue::from_str(value),
        ) {
            map.insert(name, value);
        }
    }
    map
}

fn search_id() -> String {
    // 上游用随机数与当日毫秒拼出的 64 位整数；这里用等价的伪随机拼法。
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let part1 = (nanos % 20 + 1) * 18_014_398_509_481_984;
    let part2 = ((millis ^ nanos) % 4_194_305) * 4_294_967_296;
    let part3 = millis % 86_400_000;
    (part1 + part2 + part3).to_string()
}

/// 调试用：直接返回搜索接口的原始响应正文，不做解析。
///
/// 只在 `tests/` 里使用，用于排查上游改版（例如返回空列表但 HTTP 200）。
pub async fn debug_search_raw(keyword: &str, limit: usize) -> Result<String, AppError> {
    let body = MusicuBody {
        comm: comm().await,
        request: RequestBody {
            method: "DoSearchForQQMusicMobile".to_string(),
            module: "music.search.SearchCgiService".to_string(),
            param: SearchParam {
                search_id: search_id(),
                remoteplace: "search.android.keyboard".to_string(),
                query: keyword.to_string(),
                search_type: 0,
                num_per_page: limit.clamp(1, 50) as i32,
                page_num: 1,
                highlight: 0,
                nqc_flag: 0,
                page_id: 1,
                grp: 1,
            },
        },
    };

    let response = client()
        .post(MUSICU_URL)
        .headers(headers(QQ_HEADERS))
        .json(&body)
        .send()
        .await
        .map_err(|error| map_http_error("QQ 音乐搜索", error))?;

    response
        .text()
        .await
        .map_err(|error| AppError::parse(format!("读取响应失败：{error}")))
}

/// 搜索歌曲。`limit` 为本端截断数量。
///
/// 移动端接口在短时间内连续请求会返回 `code: 2001`（风控屏蔽），此时退到桌面端
/// 接口 `DoSearchForQQMusicDesktop`，它不受这条限制，字段结构也不同。
pub async fn search(keyword: &str, limit: usize) -> Result<Vec<SongCandidate>, AppError> {
    let limit = limit.clamp(1, 50);

    match search_mobile(keyword, limit).await {
        Ok(found) => Ok(found),
        Err(SearchFailure::Blocked) => search_desktop(keyword, limit).await.map_err(Into::into),
        Err(SearchFailure::Fatal(error)) => Err(error),
    }
}

/// 移动端搜索的失败分类：屏蔽（可降级）与其他（直接报错）。
enum SearchFailure {
    Blocked,
    Fatal(AppError),
}

impl From<SearchFailure> for AppError {
    fn from(failure: SearchFailure) -> Self {
        match failure {
            SearchFailure::Blocked => AppError::upstream("QQ 音乐搜索被限流，请稍后重试"),
            SearchFailure::Fatal(error) => error,
        }
    }
}

/// 移动端搜索：结果在 `body.item_song`。
async fn search_mobile(keyword: &str, limit: usize) -> Result<Vec<SongCandidate>, SearchFailure> {
    let body = MusicuBody {
        comm: comm().await,
        request: RequestBody {
            method: "DoSearchForQQMusicMobile".to_string(),
            module: "music.search.SearchCgiService".to_string(),
            param: SearchParam {
                search_id: search_id(),
                remoteplace: "search.android.keyboard".to_string(),
                query: keyword.to_string(),
                search_type: 0,
                num_per_page: limit as i32,
                page_num: 1,
                highlight: 0,
                nqc_flag: 0,
                page_id: 1,
                grp: 1,
            },
        },
    };

    let response = client()
        .post(MUSICU_URL)
        .headers(headers(QQ_HEADERS))
        .json(&body)
        .send()
        .await
        .map_err(|error| SearchFailure::Fatal(map_http_error("QQ 音乐搜索", error)))?;

    if !response.status().is_success() {
        return Err(SearchFailure::Fatal(AppError::upstream(format!(
            "QQ 音乐搜索返回 HTTP {}",
            response.status().as_u16()
        ))));
    }

    let parsed: MusicuResponse = response
        .json()
        .await
        .map_err(|error| SearchFailure::Fatal(AppError::parse(format!("QQ 音乐搜索响应解析失败：{error}"))))?;

    if let Some(code) = parsed.code.filter(|code| *code != 0) {
        return Err(SearchFailure::Fatal(AppError::upstream(format!(
            "QQ 音乐搜索返回错误码 {code}"
        ))));
    }

    let Some(request) = parsed.request else {
        return Ok(Vec::new());
    };

    // 内层 code 才是业务状态：2001 = 请求过频被屏蔽。
    if request.code == Some(BLOCKED_CODE) {
        return Err(SearchFailure::Blocked);
    }

    let songs = request
        .data
        .and_then(|data| data.body)
        .and_then(|body| body.item_song)
        .unwrap_or_default();

    Ok(songs
        .into_iter()
        .map(|song| SongCandidate {
            source: Source::Qq,
            song_id: song.mid,
            numeric_id: song.numeric_id,
            title: song.title,
            artists: song
                .singer
                .unwrap_or_default()
                .into_iter()
                .map(|singer| singer.name)
                .collect(),
            album: song.album.map(|album| album.name).unwrap_or_default(),
            duration_ms: song.interval.map(|seconds| seconds * 1000),
        })
        .collect())
}

/// 桌面端搜索：结果在 `body.song.list`。
async fn search_desktop(keyword: &str, limit: usize) -> Result<Vec<SongCandidate>, AppError> {
    let body = serde_json::json!({
        "comm": {
            "ct": "19",
            "cv": "1859",
            "uin": "0"
        },
        "req": {
            "module": "music.search.SearchCgiService",
            "method": "DoSearchForQQMusicDesktop",
            "param": {
                "query": keyword,
                "num_per_page": limit,
                "page_num": 1,
                "search_type": 0
            }
        }
    });

    let response = client()
        .post(MUSICU_URL)
        .headers(headers(QQ_HEADERS))
        .json(&body)
        .send()
        .await
        .map_err(|error| map_http_error("QQ 音乐搜索（桌面端）", error))?;

    if !response.status().is_success() {
        return Err(AppError::upstream(format!(
            "QQ 音乐搜索返回 HTTP {}",
            response.status().as_u16()
        )));
    }

    let parsed: DesktopResponse = response
        .json()
        .await
        .map_err(|error| AppError::parse(format!("QQ 音乐搜索响应解析失败：{error}")))?;

    let songs = parsed
        .req
        .and_then(|request| request.data)
        .and_then(|data| data.body)
        .and_then(|body| body.song)
        .and_then(|song| song.list)
        .unwrap_or_default();

    Ok(songs
        .into_iter()
        .map(|song| SongCandidate {
            source: Source::Qq,
            song_id: song.mid,
            numeric_id: song.numeric_id,
            title: song.title,
            artists: song
                .singer
                .map(|singers| singers.into_iter().map(|singer| singer.name).collect())
                .or_else(|| {
                    song.singer_name
                        .map(|name| vec![name])
                        .filter(|names| !names.is_empty())
                })
                .unwrap_or_default(),
            album: song.album.map(|album| album.name).unwrap_or_default(),
            duration_ms: song.interval.map(|seconds| seconds * 1000),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 取歌词
// ---------------------------------------------------------------------------

/// 取回的原始歌词。
pub struct RawLyrics {
    /// 解密后的正文（QRC 或 LRC 文本）
    pub text: String,
    /// 是否为逐字（QRC）
    pub word_synced: bool,
    /// 翻译正文（纯文本 LRC），可能为空
    pub translation: Option<String>,
}

/// 取一首歌的歌词。QQ 有时只给逐行 LRC，此时 `word_synced = false`。
pub async fn fetch_lyrics(candidate: &SongCandidate) -> Result<RawLyrics, AppError> {
    let body = LyricsBody {
        comm: comm().await,
        request: LyricsRequestBody {
            method: "GetPlayLyricInfo".to_string(),
            module: "music.musichallSong.PlayLyricInfo".to_string(),
            param: LyricsParam {
                song_mid: candidate.song_id.clone(),
                song_id: candidate.numeric_id.unwrap_or(0),
                song_name: BASE64.encode(candidate.title.as_bytes()),
                singer_name: BASE64.encode(candidate.artist_text().as_bytes()),
                album_name: BASE64.encode(candidate.album.as_bytes()),
                interval: candidate.duration_ms.unwrap_or(0) / 1000,
                lrc_t: 0,
                qrc_t: 0,
                trans_t: 0,
                roma_t: 0,
                crypt: 1,
                ct: 19,
                cv: 2111,
                qrc: 1,
                roma: 1,
                trans: 1,
                lyric_type: 0,
            },
        },
    };

    let response = client()
        .post(MUSICU_URL)
        .headers(headers(QQ_HEADERS))
        .json(&body)
        .send()
        .await
        .map_err(|error| map_http_error("QQ 音乐取词", error))?;

    if !response.status().is_success() {
        return Err(AppError::upstream(format!(
            "QQ 音乐取词返回 HTTP {}",
            response.status().as_u16()
        )));
    }

    let parsed: LyricsResponse = response
        .json()
        .await
        .map_err(|error| AppError::parse(format!("QQ 音乐取词响应解析失败：{error}")))?;

    if let Some(code) = parsed.code.filter(|code| *code != 0) {
        return Err(AppError::upstream(format!("QQ 音乐取词返回错误码 {code}")));
    }

    let Some(data) = parsed.request.and_then(|request| request.data) else {
        return Err(AppError::no_lyrics("该曲目在 QQ 音乐没有歌词"));
    };

    let qrc_t = data.qrc_t.unwrap_or(0);
    let lrc_t = data.lrc_t.unwrap_or(0);

    // `lyric` 字段在同一响应里可能是 QRC 也可能是 LRC，靠 *_t 标记区分。
    let encrypted = data.lyric.filter(|text| !text.is_empty());
    let Some(encrypted) = encrypted else {
        return Err(AppError::no_lyrics("该曲目在 QQ 音乐没有歌词"));
    };

    let (text, word_synced) = if qrc_t != 0 {
        match decrypt_qrc(&encrypted).and_then(|xml| extract_lyric_content(&xml)) {
            Some(text) => (text, true),
            None => {
                // 标记说给了 QRC 但解不出来（占位密文），仍然当成失败。
                return Err(AppError::parse("QQ 音乐返回的逐字歌词无法解密"));
            }
        }
    } else if lrc_t != 0 {
        let text = strip_xml_envelope(&encrypted);
        (text, false)
    } else {
        return Err(AppError::no_lyrics("该曲目在 QQ 音乐没有歌词"));
    };

    let translation = data
        .trans
        .filter(|text| !text.is_empty() && data.trans_t.unwrap_or(0) != 0)
        .map(|encrypted| {
            decrypt_qrc(&encrypted)
                .or_else(|| Some(strip_xml_envelope(&encrypted)))
                .unwrap_or_default()
        })
        .filter(|text| !text.trim().is_empty());

    Ok(RawLyrics {
        text,
        word_synced,
        translation,
    })
}

/// QRC XML 节点名到用途的映射。
const XML_MAPPING: [(&str, &str); 2] = [("Lyric_1", "lyric"), ("content", "content")];

/// 从 QRC 的 XML 信封里取出 `LyricContent` 属性；不是 XML 时原样返回。
fn extract_lyric_content(raw: &str) -> Option<String> {
    if !raw.contains("<?xml") && !raw.contains("<QrcInfos") {
        return Some(raw.to_string());
    }

    let mut text = raw.to_string();
    // 封装可能嵌套多层，最多解两层。
    for _ in 0..2 {
        if !text.contains("<?xml") && !text.contains("<QrcInfos") {
            return Some(text);
        }

        let document = xml_utils::create(&text)?;
        let mut found: HashMap<String, &xml_utils::XmlNode> = HashMap::new();
        xml_utils::recursion_find_element(&document, &XML_MAPPING, &mut found);
        text = found
            .get("lyric")
            .and_then(|node| node.attribute("LyricContent"))
            .map(str::to_string)?;
    }

    Some(text)
}

/// 去掉 `<content><![CDATA[...]]></content>` 这类外层，取出正文。
fn strip_xml_envelope(raw: &str) -> String {
    if let Some(start) = raw.find("CDATA[") {
        let rest = &raw[start + 6..];
        if let Some(end) = rest.find("]]>") {
            return rest[..end].to_string();
        }
    }
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_cdata_envelope() {
        let raw = "<content><![CDATA[[00:01.00]hello]]></content>";
        assert_eq!(strip_xml_envelope(raw), "[00:01.00]hello");
    }

    #[test]
    fn plain_text_passes_through_envelope_stripping() {
        assert_eq!(strip_xml_envelope("[00:01.00]hello"), "[00:01.00]hello");
    }

    #[test]
    fn extracts_lyric_content_from_qrc_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos><QrcHeadInfo SaveTime="0" Version="100"/><LyricInfo>
<Lyric_1 LyricType="1" LyricContent="[ti:晴天][0,2250]晴(0,160)天(160,160)"/>
</LyricInfo></QrcInfos>"#;
        let content = extract_lyric_content(xml).expect("应取到 LyricContent");
        assert!(content.starts_with("[ti:晴天]"), "实际：{content}");
        assert!(content.contains("晴(0,160)"));
    }

    #[test]
    fn non_xml_input_is_returned_as_is() {
        assert_eq!(
            extract_lyric_content("[ti:晴天]\n[00:00.00]晴天").as_deref(),
            Some("[ti:晴天]\n[00:00.00]晴天")
        );
    }
}
