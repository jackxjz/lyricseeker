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

/**
 * 译文在复制结果里的摆放方式，与 Rust 侧 `TranslationLayout` 一一对应。
 *
 * - `sameLine`    `[t]原词 译文`
 * - `nextLine`    `[t]原词` 换行 `[t]译文`（默认）
 * - `before`      `[t]译文` 换行 `[t]原词`
 * - `interleaved` `[t]原词` 换行 `[下一行 t]译文`
 */
export type TranslationLayout = "sameLine" | "nextLine" | "before" | "interleaved";

export interface CopyOptions {
  includeMetadata: boolean;
  includeTranslation: boolean;
  filterCredits: boolean;
  keepSpaces: boolean;
  translationLayout: TranslationLayout;
}

/** 可以「点一下取反」的开关项。`translationLayout` 是单选，不在此列。 */
export type CopyToggleKey = Exclude<keyof CopyOptions, "translationLayout">;

/**
 * 界面主题。
 *
 * - `system` 跟随 Windows 的浅色/深色设置（默认）
 * - `light`  强制浅色
 * - `dark`   强制深色
 *
 * 决定方式：给 `<html>` 挂 `data-theme`，CSS 里的 `light-dark(浅, 深)`
 * 只看 `color-scheme`，所以真正生效的是 `color-scheme` 的取值。
 */
export type ThemeMode = "system" | "light" | "dark";

/** 主题三个切换卡，顺序即界面顺序。 */
export const THEME_MODE_ORDER: ThemeMode[] = ["system", "light", "dark"];

export const THEME_MODE_LABEL: Record<ThemeMode, string> = {
  system: "跟随系统",
  light: "浅色",
  dark: "深色",
};

/** 翻译板块的单选项，顺序即界面顺序。 */
export const TRANSLATION_LAYOUT_ORDER: TranslationLayout[] = [
  "sameLine",
  "nextLine",
  "before",
  "interleaved",
];

export const TRANSLATION_LAYOUT_LABEL: Record<TranslationLayout, string> = {
  sameLine: "同一行",
  nextLine: "原文在上",
  before: "译文在上",
  interleaved: "交错",
};

/** 每个选项配一份「输出长什么样」，比文字说明更省解释。用换行代替分隔符。 */
export const TRANSLATION_LAYOUT_SAMPLE: Record<TranslationLayout, string> = {
  sameLine: "[00:01.168]原词 翻译词",
  nextLine: "[00:01.168]原词\n[00:01.168]翻译词",
  before: "[00:01.168]翻译词\n[00:01.168]原词",
  interleaved: "[00:01.168]原词1\n[00:02.268]翻译词1\n[00:02.268]原词2",
};

export interface GeneratedText {
  text: string;
  lineCount: number;
  charCount: number;
}

export interface CacheInfo {
  location: string;
  ttlSeconds: number;
  /** 当前占用字节数。 */
  bytes: number;
  /** 条目数。 */
  files: number;
}

/**
 * 复制的默认选项。
 *
 * 默认打开三项（翻译、元信息、空格）是用户的实际用法：拿到歌词后几乎
 * 总是直接粘到播放器里，缺了翻译和标签就要手动补。「过滤制作信息」
 * 默认关闭 —— 它是破坏性的，误开会让用户以为歌词丢了。
 */
export const DEFAULT_COPY_OPTIONS: CopyOptions = {
  includeTranslation: true,
  includeMetadata: true,
  keepSpaces: true,
  filterCredits: false,
  // 默认第 2 种（译文另起一行跟在原文后面），这也是老版本的行为，升级不改变结果。
  translationLayout: "nextLine",
};

export const SOURCE_LABEL: Record<Source, string> = {
  qq: "QQ 音乐",
  kugou: "酷狗音乐",
};

export const FORMAT_LABEL: Record<CopyFormat, string> = {
  plain: "逐字（普通 LRC）",
  enhanced: "逐字（增强 LRC）",
  lrc: "逐行（带时间戳）",
  text: "纯文本",
};

/** 复制位的顺序：Ctrl+1..4 与界面按钮都用它，避免两处各写一份。 */
export const FORMAT_ORDER: CopyFormat[] = ["plain", "enhanced", "lrc", "text"];

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
