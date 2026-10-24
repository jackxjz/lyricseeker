/**
 * 界面用的小图标，全部内联 SVG：
 * 不引图标库、不做外链请求，跟着 currentColor 走颜色，
 * 在 WebView2 里也不会有字体图标首帧空白的问题。
 */

interface IconProps {
  /** 边长（px）。默认 16，正好配 13px 正文的视觉重量。 */
  size?: number;
}

/** 图钉：置顶 / 取消置顶。 */
export function PinIcon({ size = 16 }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M12 17v5" />
      <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />
    </svg>
  );
}

/** 齿轮：设置。 */
export function GearIcon({ size = 16 }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <circle cx="12" cy="12" r="3.1" />
      <path d="M19.4 13.5a7.6 7.6 0 0 0 0-3l1.6-1.2-1.6-2.8-1.9.8a7.4 7.4 0 0 0-2.6-1.5L14.6 3H9.4l-.3 1.8a7.4 7.4 0 0 0-2.6 1.5l-1.9-.8L3 8.3l1.6 1.2a7.6 7.6 0 0 0 0 3L3 13.7l1.6 2.8 1.9-.8a7.4 7.4 0 0 0 2.6 1.5l.3 1.8h5.2l.3-1.8a7.4 7.4 0 0 0 2.6-1.5l1.9.8 1.6-2.8-1.6-1.2z" />
    </svg>
  );
}
