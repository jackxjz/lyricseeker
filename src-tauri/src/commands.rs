//! Tauri 命令层：前端能调用的全部接口。

use tauri::{Manager, State};

use crate::cache;
use crate::format::{self, CopyOptions};
use crate::lyrics;
use crate::model::{AppError, LyricsDoc, SongCandidate, Source};
use crate::providers::{self, kugou, qq};

/// 搜索结果上限：单端最多返回 30 条，足够用户挑选。
const MAX_RESULTS: usize = 30;

/// 搜索歌曲。
///
/// - `source`：主页 tab 的选择，`"qq"` / `"kugou"` / `"all"`（或空 = 全部）。
/// - `enabled_sources`：设置页里打开的歌词源。关掉的源即使 tab 选了也不会请求。
///
/// 两者取交集；交集为空时报出可读的错误，而不是静默返回空列表。
#[tauri::command]
pub async fn search(
    source: String,
    keyword: String,
    limit: Option<u32>,
    enabled_sources: Option<Vec<String>>,
) -> Result<Vec<SongCandidate>, AppError> {
    let all_enabled = enabled_sources.unwrap_or_else(|| {
        vec![Source::Qq.as_str().to_string(), Source::Kugou.as_str().to_string()]
    });
    let enabled: Vec<Source> = all_enabled
        .iter()
        .filter_map(|name| Source::parse(name))
        .collect();

    if enabled.is_empty() {
        return Err(AppError::new(
            "Config",
            "所有歌词源都已关闭，请在设置里至少开启一个",
        ));
    }

    let sources: Vec<Source> = if source.trim().is_empty() || source.eq_ignore_ascii_case("all") {
        enabled.clone()
    } else {
        let picked = Source::parse(&source)
            .ok_or_else(|| AppError::new("Config", format!("未知的音乐来源：{source}")))?;
        if !enabled.contains(&picked) {
            return Err(AppError::new(
                "Config",
                format!(
                    "{} 已在设置里关闭，请先在设置里开启或切换到其它来源",
                    picked.label()
                ),
            ));
        }
        vec![picked]
    };

    let limit = limit
        .map(|value| value as usize)
        .unwrap_or(MAX_RESULTS)
        .clamp(1, MAX_RESULTS);

    let mut results = providers::search(sources, &keyword, limit).await?;
    results.truncate(limit);
    Ok(results)
}

/// 取歌词。命中缓存时直接返回，并在 `fromCache` 标记。
///
/// `force_refresh` 为真时跳过缓存，用于用户主动「重新获取」。
#[tauri::command]
pub async fn fetch_lyrics(
    candidate: SongCandidate,
    force_refresh: Option<bool>,
) -> Result<LyricsDoc, AppError> {
    let force = force_refresh.unwrap_or(false);

    if !force {
        if let Some(doc) = cache::load(candidate.source, &candidate.song_id) {
            log::info!("歌词缓存命中：{} {}", candidate.artist_text(), candidate.title);
            return Ok(doc);
        }
    }

    let raw = match candidate.source {
        Source::Qq => {
            let raw = qq::fetch_lyrics(&candidate).await?;
            let mut parsed = lyrics::parse_qq(&raw.text, raw.word_synced);
            if let Some(translation) = raw.translation.as_deref() {
                lyrics::attach_translation(&mut parsed, translation);
            }
            parsed
        }
        Source::Kugou => {
            let raw = kugou::fetch_lyrics(&candidate).await?;
            let mut parsed = lyrics::parse_kugou(&raw.text, raw.word_synced);
            if let Some(translations) = raw.translation_lines.as_deref() {
                lyrics::attach_translation_lines(&mut parsed, translations);
            }
            parsed
        }
    };

    // 一首歌连一行都没有，视为上游没有提供歌词，而不是成功返回空文档。
    if raw.lines.is_empty() {
        return Err(AppError::no_lyrics(format!(
            "「{}」在{}没有可用歌词",
            candidate.title,
            candidate.source.label()
        )));
    }

    let doc = lyrics::into_doc(candidate, raw);
    cache::save(&doc);
    Ok(doc)
}

/// 按格式生成文本，供前端写入剪贴板。
///
/// 返回生成好的字符串（前端不再重复实现时间格式），同时带回行数以做 Toast 文案。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedText {
    pub text: String,
    pub line_count: usize,
    pub char_count: usize,
}

#[tauri::command]
pub fn generate_text(
    doc: LyricsDoc,
    format_name: String,
    options: Option<CopyOptions>,
) -> Result<GeneratedText, AppError> {
    let options = options.unwrap_or_default();
    let text = format::generate(&doc, &format_name, &options).ok_or_else(|| {
        AppError::new("Config", format!("未知的复制格式：{format_name}"))
    })?;

    if text.trim().is_empty() {
        return Err(AppError::no_lyrics("该歌曲没有可复制的内容"));
    }

    let line_count = text.lines().filter(|line| !line.trim().is_empty()).count();
    let char_count = text.chars().count();
    Ok(GeneratedText {
        text,
        line_count,
        char_count,
    })
}

/// 清空缓存，返回删除的条数。
#[tauri::command]
pub fn clear_cache() -> Result<u64, AppError> {
    cache::clear()
}

/// 缓存的磁盘位置与有效期，供设置页提示。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub location: String,
    pub ttl_seconds: u64,
    /// 当前占用字节数。
    pub bytes: u64,
    /// 条目数。
    pub files: u64,
}

#[tauri::command]
pub fn cache_info() -> CacheInfo {
    let stats = cache::stats();
    CacheInfo {
        location: cache::location(),
        ttl_seconds: cache::TTL_SECS,
        bytes: stats.bytes,
        files: stats.files,
    }
}

/// 切换窗口置顶。
#[tauri::command]
pub fn set_always_on_top(window: tauri::Window, on: bool) -> Result<(), AppError> {
    window
        .set_always_on_top(on)
        .map_err(|error| AppError::new("Window", format!("设置窗口置顶失败：{error}")))
}

/// 把窗口从隐藏状态恢复出来并聚焦（全局快捷键用）。
pub fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 隐藏主窗口（全局快捷键二次触发时）。
pub fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// 占位：确保 `State` 被引用，避免未来加状态时忘记引入。
#[allow(dead_code)]
fn _assert_state_available(_: State<'_, ()>) {}
