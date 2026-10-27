import type { CopyOptions, CopyToggleKey, Source, ThemeMode, TranslationLayout } from "../types";
import {
  SOURCE_LABEL,
  THEME_MODE_LABEL,
  THEME_MODE_ORDER,
  TRANSLATION_LAYOUT_LABEL,
  TRANSLATION_LAYOUT_ORDER,
  TRANSLATION_LAYOUT_SAMPLE,
} from "../types";
import { formatBytes } from "../settings";

interface SettingsPageProps {
  enabledSources: Source[];
  copy: CopyOptions;
  theme: ThemeMode;
  cacheBytes: number;
  cacheFiles: number;
  cacheLocation: string;
  cacheBusy: boolean;
  onToggleSource: (source: Source) => void;
  onToggleCopy: (key: CopyToggleKey) => void;
  onSelectTranslation: (layout: TranslationLayout) => void;
  onSelectTheme: (mode: ThemeMode) => void;
  onClearCache: () => void;
  onClose: () => void;
}

const SOURCE_HINTS: Record<Source, string> = {
  qq: "逐字优先走 QRC，没有逐字时退回逐行 LRC。",
  kugou: "逐字走 KRC，通常带译文。",
};

/** 「过滤制作信息」默认关闭：它会删掉「词：周杰伦」这类行，误开像丢歌词。 */
const COPY_ITEMS: Array<{
  key: CopyToggleKey;
  label: string;
  hint: string;
}> = [
  { key: "includeTranslation", label: "带翻译", hint: "在每行原文后面附一行译文。" },
  { key: "includeMetadata", label: "含 [ti:] / [ar:] / [al:] 标签", hint: "复制时在开头写歌名、歌手、专辑。" },
  { key: "keepSpaces", label: "保留逐字中的空格", hint: "关掉后字与字之间不再用空格对齐。" },
  {
    key: "filterCredits",
    label: "过滤「词/曲/编曲」等制作信息",
    hint: "会删掉制作人员那几行，默认关闭以免误删。",
  },
];

export function SettingsPage({
  enabledSources,
  copy,
  theme,
  cacheBytes,
  cacheFiles,
  cacheLocation,
  cacheBusy,
  onToggleSource,
  onToggleCopy,
  onSelectTranslation,
  onSelectTheme,
  onClearCache,
  onClose,
}: SettingsPageProps) {
  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-page" onClick={(event) => event.stopPropagation()}>
        <header className="settings-head">
          <h2>设置</h2>
          <button type="button" className="ghost-btn" onClick={onClose} title="关闭（Esc）">
            关闭
          </button>
        </header>

        <div className="settings-body">
          {/* 主题放最前面：它是整页唯一影响观感的项，翻在下面容易被忽略。 */}
          <section className="settings-section">
            <h3>主题</h3>
            <p className="settings-desc">「跟随系统」会随着 Windows 的浅色 / 深色自动切换。</p>
            <ul className="settings-list settings-themes">
              {THEME_MODE_ORDER.map((mode) => (
                <li key={mode}>
                  <label className={`layout-row theme-row${theme === mode ? " is-checked" : ""}`}>
                    <span className="layout-head">
                      <input
                        type="radio"
                        name="theme-mode"
                        value={mode}
                        checked={theme === mode}
                        onChange={() => onSelectTheme(mode)}
                      />
                      <span className="layout-name">{THEME_MODE_LABEL[mode]}</span>
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          </section>

          <section className="settings-section">
            <h3>词源</h3>
            <p className="settings-desc">关掉的歌词源不会出现在主页，也不会被检索。</p>
            <ul className="settings-list">
              {(Object.keys(SOURCE_LABEL) as Source[]).map((source) => {
                const on = enabledSources.includes(source);
                return (
                  <li key={source}>
                    <label className="settings-row">
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() => onToggleSource(source)}
                      />
                      <span className="settings-label">
                        {SOURCE_LABEL[source]}
                        <em>{SOURCE_HINTS[source]}</em>
                      </span>
                    </label>
                  </li>
                );
              })}
            </ul>
            {enabledSources.length === 0 && (
              <p className="settings-warn">至少开启一个歌词源，否则无法检索。</p>
            )}
          </section>

          <section className="settings-section">
            <h3>歌词</h3>
            <p className="settings-desc">这些选项直接决定 Ctrl+1..4 复制出去的内容。</p>
            <ul className="settings-list">
              {COPY_ITEMS.map((item) => (
                <li key={item.key}>
                  <label className="settings-row">
                    <input
                      type="checkbox"
                      checked={copy[item.key]}
                      onChange={() => onToggleCopy(item.key)}
                    />
                    <span className="settings-label">
                      {item.label}
                      <em>{item.hint}</em>
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          </section>

          {/* 翻译板块紧跟在「歌词」下面：它管的是同一个开关的下一层细节，
              放到别处会让人以为翻译是独立功能。关掉「带翻译」时整段置灰，
              因为这时无论选哪种摆放都不会进入复制结果。 */}
          <section
            className={`settings-section${copy.includeTranslation ? "" : " is-disabled"}`}
            aria-disabled={!copy.includeTranslation}
          >
            <h3>翻译</h3>
            <p className="settings-desc">
              决定译文摆在哪一行。这里选成什么样，复制出去就是什么样。
            </p>
            <ul className="settings-list settings-layouts">
              {TRANSLATION_LAYOUT_ORDER.map((layout) => {
                const disabled = !copy.includeTranslation;
                return (
                  <li key={layout}>
                    <label className={`layout-row${disabled ? " is-disabled" : ""}`}>
                      <span className="layout-head">
                        <input
                          type="radio"
                          name="translation-layout"
                          value={layout}
                          checked={copy.translationLayout === layout}
                          disabled={disabled}
                          onChange={() => onSelectTranslation(layout)}
                        />
                        <span className="layout-name">{TRANSLATION_LAYOUT_LABEL[layout]}</span>
                      </span>
                      <em>{TRANSLATION_LAYOUT_SAMPLE[layout]}</em>
                    </label>
                  </li>
                );
              })}
            </ul>
            {!copy.includeTranslation && (
              <p className="settings-warn">先在上面勾选「带翻译」，这里才能选。</p>
            )}
          </section>

          <section className="settings-section">
            <h3>缓存</h3>
            <p className="settings-desc">歌词按歌曲缓存，重复查看不必重新请求上游。</p>
            <div className="settings-cache">
              <div className="cache-meta">
                <strong>{formatBytes(cacheBytes)}</strong>
                <span>
                  {cacheFiles} 个条目 · {cacheLocation}
                </span>
              </div>
              <button
                type="button"
                className="ghost-btn danger"
                onClick={onClearCache}
                disabled={cacheBusy || cacheFiles === 0}
              >
                {cacheBusy ? "清理中…" : "清理缓存"}
              </button>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
