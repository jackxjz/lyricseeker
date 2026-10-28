"""生成 LyricSeeker 的应用图标。

设计：圆角方形渐变底 + 白色音符 + 三段「逐字」分块歌词行。
输出 1024x1024 透明边角的 PNG，再交给 `npx tauri icon` 生成全套尺寸。

用法: python scripts/make_icon.py <输出 png 路径>
"""

from __future__ import annotations

import sys

from PIL import Image, ImageDraw, ImageFilter

S = 1024  # 画布边长（超采样后缩小，边缘更干净）
SS = 2  # 超采样倍数
W = S * SS

# 配色跟应用主色一致
TOP = (86, 141, 255)  # #568dff
BOTTOM = (37, 92, 208)  # #255cd0
WHITE = (255, 255, 255)


def lerp(a: int, b: int, t: float) -> int:
    return round(a + (b - a) * t)


def vertical_gradient(size: int, top: tuple[int, int, int], bottom: tuple[int, int, int]) -> Image.Image:
    grad = Image.new("RGB", (1, size))
    px = grad.load()
    for y in range(size):
        t = y / max(size - 1, 1)
        # 轻微非线性，底部颜色更沉一点，避免上下两截感
        t = t**0.85
        px[0, y] = (
            lerp(top[0], bottom[0], t),
            lerp(top[1], bottom[1], t),
            lerp(top[2], bottom[2], t),
        )
    return grad.resize((size, size), Image.BILINEAR)


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def draw_note(draw: ImageDraw.ImageDraw) -> None:
    """左下角画一个四分音符：符头 + 符干。"""
    hx, hy = W * 0.300, W * 0.690
    rx, ry = W * 0.082, W * 0.062
    draw.ellipse((hx - rx, hy - ry, hx + rx, hy + ry), fill=WHITE)

    stem_w = W * 0.052
    stem_x = hx + rx * 0.62
    draw.rounded_rectangle(
        (stem_x, W * 0.500, stem_x + stem_w, hy),
        radius=stem_w / 2,
        fill=WHITE,
    )


def draw_lyric_bars(draw: ImageDraw.ImageDraw) -> None:
    """上方两整行 + 右下四个字块，呼应「逐字」。"""
    left = W * 0.225
    right = W * 0.775
    height = W * 0.060
    radius = height / 2

    for y_ratio, end_x in ((0.285, right), (0.400, W * 0.640)):
        y = W * y_ratio
        draw.rounded_rectangle((left, y, end_x, y + height), radius=radius, fill=WHITE)

    # 右边的四个方块，看起来像「逐字」被切开
    y = W * 0.610
    blocks = 4
    gap = W * 0.020
    block_left = W * 0.470
    block_w = (right - block_left - gap * (blocks - 1)) / blocks
    for i in range(blocks):
        x0 = block_left + i * (block_w + gap)
        # 圆角只给一点点，才像「字块」而不是圆点
        draw.rounded_rectangle(
            (x0, y, x0 + block_w, y + height), radius=height * 0.26, fill=WHITE
        )


def main() -> None:
    out = sys.argv[1] if len(sys.argv) > 1 else "app-icon.png"

    bg = vertical_gradient(W, TOP, BOTTOM).convert("RGBA")
    bg.putalpha(rounded_mask(W, radius=int(W * 0.225)))

    layer = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    draw_note(draw)
    draw_lyric_bars(draw)

    # 图案整体略微放大并居中：手摆的坐标容易偏上，统一在这里校准
    SCALE = 1.12
    bbox = layer.getbbox()
    content = layer.crop(bbox)
    new_size = (int(content.width * SCALE), int(content.height * SCALE))
    content = content.resize(new_size, Image.LANCZOS)
    layer = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    layer.alpha_composite(
        content,
        ((W - new_size[0]) // 2, int((W - new_size[1]) / 2)),
    )

    # 极轻的投影，让白块在浅色背景上也不糊在一起
    shadow = layer.filter(ImageFilter.GaussianBlur(W * 0.012))
    shadow.putalpha(shadow.getchannel("A").point(lambda v: int(v * 0.22)))
    canvas = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    canvas.alpha_composite(bg)
    canvas.alpha_composite(shadow, (0, int(W * 0.008)))
    canvas.alpha_composite(layer)

    canvas.resize((S, S), Image.LANCZOS).save(out)
    print("saved", out)


if __name__ == "__main__":
    main()
