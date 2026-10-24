import { invoke } from "@tauri-apps/api/core";
import type {
  CacheInfo,
  CopyFormat,
  CopyOptions,
  GeneratedText,
  LyricsDoc,
  SongCandidate,
  Source,
  SourceFilter,
} from "./types";

/**
 * 搜索歌曲。
 *
 * `source` 是主页 tab 的选择（`"all"` 表示不限定），`enabled` 是设置里
 * 打开的源；两者由后端取交集，被关掉的源不会被请求。
 */
export function searchSongs(
  source: SourceFilter,
  keyword: string,
  enabled: Source[],
  limit = 30,
): Promise<SongCandidate[]> {
  return invoke<SongCandidate[]>("search", { source, keyword, limit, enabledSources: enabled });
}

/** 取歌词；命中缓存直接返回。 */
export function fetchLyrics(
  candidate: SongCandidate,
  forceRefresh = false,
): Promise<LyricsDoc> {
  return invoke<LyricsDoc>("fetch_lyrics", { candidate, forceRefresh });
}

/** 在 Rust 侧生成待复制的文本。 */
export function generateText(
  doc: LyricsDoc,
  formatName: CopyFormat,
  options: CopyOptions,
): Promise<GeneratedText> {
  return invoke<GeneratedText>("generate_text", { doc, formatName, options });
}

export function clearCache(): Promise<number> {
  return invoke<number>("clear_cache");
}

export function getCacheInfo(): Promise<CacheInfo> {
  return invoke<CacheInfo>("cache_info");
}

export function setAlwaysOnTop(on: boolean): Promise<void> {
  return invoke<void>("set_always_on_top", { on });
}
