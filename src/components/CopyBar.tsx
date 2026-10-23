import { useState } from "react";
import type { CopyFormat, CopyOptions, LyricsDoc } from "../types";
import { FORMAT_LABEL } from "../types";

interface Props {
  doc: LyricsDoc | null;
  options: CopyOptions;
  busy: boolean;
  disabled: boolean;
  onOptionsChange: (options: CopyOptions) => void;
  onCopy: (formatName: CopyFormat) => void;
  onRefresh: () => void;
  onClearCache: () => void;
  cacheLocation: string | null;
}

const FORMATS: { value: CopyFormat; hint: string; shortcut: string }[] = [
  { value: "enhanced", hint: "行时间戳 + 每字 <mm:ss.mmm>", shortcut: "Ctrl+1" },
  { value: "raw", hint: "平台原始逐字记号（QRC / KRC）", shortcut: "Ctrl+2" },
  { value: "lrc", hint: "标准 LRC，一行一条", shortcut: "Ctrl+3" },
  { value: "text", hint: "不带任何时间标记", shortcut: "Ctrl+4" },
];

const OPTION_LABELS: { key: keyof CopyOptions; label: string; hint: string }[] = [
  { key: "includeMetadata", label: "含 [ti:]/[ar:]/[al:] 标签", hint: "把歌曲元信息写进复制内容" },
  { key: "includeTranslation", label: "含翻译", hint: "把翻译行附在每行原词后" },
  { key: "filterCredits", label: "过滤「词/曲/编曲」等制作信息", hint: "去掉开头的制作人员名单" },
  { key: "keepSpaces", label: "保留逐字中的空格", hint: "关闭后会把整行空白音节去掉" },
];

export function CopyBar({
  doc,
  options,
  busy,
  disabled,
  onOptionsChange,
  onCopy,
  onRefresh,
  onClearCache,
  cacheLocation,
}: Props) {
  const [showOptions, setShowOptions] = useState(false);

  return (
    <footer className="copybar">
      <div className="copy-buttons">
        {FORMATS.map((format) => (
          <button
            key={format.value}
            type="button"
            className="copy-button"
            disabled={disabled || busy}
            title={`${format.hint} · ${format.shortcut}`}
            onClick={() => onCopy(format.value)}
          >
            <span className="copy-label">{FORMAT_LABEL[format.value]}</span>
            <span className="copy-shortcut">{format.shortcut}</span>
          </button>
        ))}
      </div>

      <div className="copy-side">
        <button
          type="button"
          className={showOptions ? "ghost active" : "ghost"}
          disabled={disabled}
          onClick={() => setShowOptions(!showOptions)}
        >
          选项
        </button>
        <button
          type="button"
          className="ghost"
          disabled={!doc || busy}
          title="忽略缓存，重新向上游请求"
          onClick={onRefresh}
        >
          重新获取
        </button>
        <button
          type="button"
          className="ghost"
          title={cacheLocation ? `缓存目录：${cacheLocation}` : "清除本地歌词缓存"}
          onClick={onClearCache}
        >
          清缓存
        </button>
      </div>

      {showOptions ? (
        <div className="options-panel">
          {OPTION_LABELS.map((option) => (
            <label key={option.key} className="option-row">
              <input
                type="checkbox"
                checked={options[option.key]}
                onChange={(event) =>
                  onOptionsChange({ ...options, [option.key]: event.target.checked })
                }
              />
              <span className="option-text">
                <span className="option-label">{option.label}</span>
                <span className="option-hint">{option.hint}</span>
              </span>
            </label>
          ))}
        </div>
      ) : null}
    </footer>
  );
}
