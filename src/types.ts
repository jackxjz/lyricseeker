// 与 Rust 侧 `src-tauri/src/model.rs` 一一对应的类型定义。
// 字段名全部为 camelCase（Rust 侧 `#[serde(rename_all = "camelCase")]`）。

export type Source = "qq" | "kugou";
export type SourceFilter = Source | "all";
export type RawFmt = "qrc" | "krc" | "lrc";
export type CopyFormat = "enhanced" | "plain" | "lrc" | "text";

export interface SongCandidate {
  source: Source;
  songId: string;
  numericId: number | null;
  title: string;
  artists: string[];
  album: string;
  durationMs: number | null;
}

export interface Syllable {
  text: string;
  startMs: number;
  durMs: number;
}

export interface LyricLine {
  startMs: number;
  durMs: number | null;
  text: string;
  syllables: Syllable[] | null;
  translation: string | null;
}

export interface LyricsDoc {
  song: SongCandidate;
  rawFmt: RawFmt;
  lines: LyricLine[];
  wordSynced: boolean;
  fromCache: boolean;
  notice: string | null;
}

export interface AppError {
  kind: string;
  message: string;
}

export interface CopyOptions {
  includeMetadata: boolean;
  includeTranslation: boolean;
  filterCredits: boolean;
  keepSpaces: boolean;
}

export interface GeneratedText {
  text: string;
  lineCount: number;
  charCount: number;
}

export interface CacheInfo {
  location: string;
  ttlSeconds: number;
}

export const DEFAULT_COPY_OPTIONS: CopyOptions = {
  includeMetadata: false,
  includeTranslation: false,
  filterCredits: false,
  keepSpaces: true,
};

export const SOURCE_LABEL: Record<Source, string> = {
  qq: "QQ 音乐",
  kugou: "酷狗音乐",
};

export const FORMAT_LABEL: Record<CopyFormat, string> = {
  enhanced: "逐字（增强 LRC）",
  plain: "逐字（普通 LRC）",
  lrc: "逐行（带时间戳）",
  text: "纯文本",
};

/** 把毫秒格式化成 m:ss，用于候选列表展示。 */
export function formatDuration(ms: number | null): string {
  if (ms === null || ms <= 0) return "--:--";
  const totalSeconds = Math.round(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

/** 判断一个值是不是 Rust 抛回的 AppError。 */
export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).kind === "string" &&
    typeof (value as AppError).message === "string"
  );
}

/** 把任意异常归一成可展示的中文消息。 */
export function describeError(error: unknown): string {
  if (isAppError(error)) return error.message;
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "发生未知错误";
}
