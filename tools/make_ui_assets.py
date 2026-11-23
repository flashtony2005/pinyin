"""生成新界面所需的渐变背景与图标（纯 PIL，无第三方字体依赖）。
输出：
  assets/bg.png        1080x2248 竖向浅紫渐变（顶部偏蓝紫 -> 底部偏粉紫）
  assets/wave.png      声波图标（紫渐变竖条）
  assets/speaker.png   喇叭图标（紫色）
  assets/clear.png     清除/重置图标（灰紫圆形箭头）
"""
import os
from PIL import Image, ImageDraw

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets")
os.makedirs(OUT, exist_ok=True)


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def gradient_bg():
    w, h = 1080, 2248
    # 三段渐变：顶(淡蓝紫) -> 中(淡紫) -> 底(淡粉紫)
    top = (0xEE, 0xF1, 0xFF)
    mid = (0xF3, 0xEE, 0xFE)
    bot = (0xFD, 0xEE, 0xF6)
    img = Image.new("RGB", (1, h))
    px = img.load()
    for y in range(h):
        t = y / (h - 1)
        if t < 0.55:
            px[0, y] = lerp(top, mid, t / 0.55)
        else:
            px[0, y] = lerp(mid, bot, (t - 0.55) / 0.45)
    img = img.resize((w, h))
    # 顶部左侧加一团柔和蓝紫光晕，底部右侧加一团粉光晕，贴近截图质感
    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for cy, col, r in [(-120, (150, 170, 255, 70), 620), (2120, (255, 160, 210, 55), 560)]:
        for i in range(r, 0, -6):
            a = int(col[3] * (1 - i / r) ** 1.6)
            gd.ellipse([-r // 2 + (w - r) // 2, cy - i // 2, (w - r) // 2 + r // 2 + r, cy + i // 2],
                       fill=(col[0], col[1], col[2], a))
    img = Image.alpha_composite(img.convert("RGBA"), glow).convert("RGB")
    img.save(os.path.join(OUT, "bg.png"))
    print("bg.png", img.size)


def wave_icon():
    # 声波：5 根竖条，中间高两边低，紫渐变
    S = 120
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    bars = [(20, 34, 86), (42, 16, 104), (64, 6, 114), (86, 20, 100), (108, 40, 80)]
    c_top = (0x8B, 0x5C, 0xF5)
    c_bot = (0x5B, 0x7C, 0xF2)
    for x, y0, y1 in bars:
        for y in range(y0, y1):
            t = (y - y0) / max(1, (y1 - y0))
            d.line([(x, y), (x, y)], fill=lerp(c_top, c_bot, t) + (255,), width=12)
    img.save(os.path.join(OUT, "wave.png"))
    print("wave.png", img.size)


def speaker_icon():
    S = 120
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    col = (0x6C, 0x4F, 0xF5, 255)
    # 喇叭主体：梯形 + 小方块
    d.polygon([(20, 46), (48, 46), (78, 24), (78, 96), (48, 74), (20, 74)], fill=col)
    d.rectangle([14, 44, 24, 76], fill=col)
    # 两道声波弧线
    d.arc([80, 38, 108, 82], start=-55, end=55, fill=col, width=8)
    d.arc([92, 26, 128, 94], start=-55, end=55, fill=col, width=8)
    img.save(os.path.join(OUT, "speaker.png"))
    print("speaker.png", img.size)


def clear_icon():
    S = 120
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    col = (0x8A, 0x86, 0x9E, 255)
    # 逆时针箭头弧
    d.arc([26, 26, 94, 94], start=40, end=320, fill=col, width=10)
    # 箭头三角（左上）
    d.polygon([(20, 58), (44, 50), (40, 76)], fill=col)
    img.save(os.path.join(OUT, "clear.png"))
    print("clear.png", img.size)


if __name__ == "__main__":
    gradient_bg()
    wave_icon()
    speaker_icon()
    clear_icon()
