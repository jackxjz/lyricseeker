// 设置项的持久化：写在 app_data_dir 的 settings.json 里（tauri-plugin-store）。
//
// 四块内容：
// - 主题：跟随系统 / 浅色 / 深色
// - 词源：哪些平台参与检索（关掉的源在主页连 tab 都不显示）
// - 歌词：复制时的四个开关
// - 缓存：只有「清理」一个动作，占用大小由后端 `cache_info` 实时给出

import { LazyStore } from "@tauri-apps/plugin-store";
import type { CopyOptions, Source, ThemeMode, TranslationLayout } from "./types";
import {
  DEFAULT_COPY_OPTIONS,
  THEME_MODE_ORDER,
  TRANSLATION_LAYOUT_ORDER,
} from "./types";

export interface Settings {
  /** 界面主题。 */
  theme: ThemeMode;
  /** 开启的歌词源。空数组视为「用户把全部关掉了」，由界面提示至少开一个。 */
  enabledSources: Source[];
  /** 复制选项。 */
  copy: CopyOptions;
}

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  enabledSources: ["qq", "kugou"],
  copy: DEFAULT_COPY_OPTIONS,
};

const STORE_PATH = "settings.json";

// autoSave 用默认的 100ms 防抖：拖动/连点不会把磁盘写爆，
// 进程被杀掉的最坏损失也只有最后 100ms 内的改动。
const store = new LazyStore(STORE_PATH, { autoSave: 100 });

/** 读取设置。任何一步失败都退回默认值 —— 设置读不出来不该拦住用户用软件。 */
export async function loadSettings(): Promise<Settings> {
  try {
    const theme = await store.get<ThemeMode>("theme");
    const enabled = await store.get<Source[]>("enabledSources");
    const copy = await store.get<CopyOptions>("copy");
    return {
      theme: normalizeTheme(theme),
      enabledSources: normalizeSources(enabled),
      copy: { ...DEFAULT_COPY_OPTIONS, ...(copy ?? {}), translationLayout: normalizeLayout((copy ?? {}).translationLayout) },
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

/** 写回设置。失败只打日志，不打断交互。 */
export async function saveSettings(settings: Settings): Promise<void> {
  try {
    await store.set("theme", settings.theme);
    await store.set("enabledSources", settings.enabledSources);
    await store.set("copy", settings.copy);
  } catch (error) {
    console.error("保存设置失败", error);
  }
}

/** 老版本设置文件没有 `theme`，或写坏了 —— 一律退回「跟随系统」。 */
function normalizeTheme(value: unknown): ThemeMode {
  return THEME_MODE_ORDER.includes(value as ThemeMode) ? (value as ThemeMode) : DEFAULT_SETTINGS.theme;
}

/** 过滤掉不认识的值、去重、保持 qq/kugou 的固定顺序。 */
function normalizeSources(value: unknown): Source[] {
  if (!Array.isArray(value)) return DEFAULT_SETTINGS.enabledSources;
  const known: Source[] = ["qq", "kugou"];
  return known.filter((source) => value.includes(source));
}

/** 老版本设置文件没有 `translationLayout`，或写坏了 —— 一律退回默认的第 2 种。 */
function normalizeLayout(value: unknown): TranslationLayout {
  return TRANSLATION_LAYOUT_ORDER.includes(value as TranslationLayout)
    ? (value as TranslationLayout)
    : DEFAULT_COPY_OPTIONS.translationLayout;
}

/** 把字节数格式化成便于阅读的大小，例如 `1.2 MB`。 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // 字节数不带小数（`512 B` 比 `512.0 B` 自然），其余保留一位。
  const digits = unit === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}
