"""Generate the app icon: a purple rounded square with a tone-marked syllable.

Run: python tools/make_icon.py
Writes assets/icon.png (1024x1024), which nsh.json names as the app's icon.
"""

import os

from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
BG_TOP = (129, 108, 247)
BG_BOTTOM = (95, 70, 234)
RADIUS = 224
OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "icon.png")
FONT_CANDIDATES = [
    r"C:\Windows\Fonts\seguisb.ttf",
    r"C:\Windows\Fonts\msyhbd.ttc",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
]


def gradient(size, top, bottom):
    img = Image.new("RGB", (size, size), top)
    px = img.load()
    for y in range(size):
        t = y / (size - 1)
        blended = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        for x in range(size):
            px[x, y] = blended
    return img


def rounded_mask(size, radius):
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return mask


def load_font(size):
    for path in FONT_CANDIDATES:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def main():
    base = gradient(SIZE, BG_TOP, BG_BOTTOM).convert("RGBA")
    base.putalpha(rounded_mask(SIZE, RADIUS))

    draw = ImageDraw.Draw(base)
    # A soft highlight in the upper left, so the icon reads as a physical tile.
    glow = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(glow).ellipse([-260, -420, 900, 260], fill=42)
    base.paste(Image.new("RGBA", (SIZE, SIZE), (255, 255, 255, 255)), (0, 0), glow)

    text = "bā"
    font = load_font(560)
    box = draw.textbbox((0, 0), text, font=font)
    x = (SIZE - (box[2] - box[0])) / 2 - box[0]
    y = (SIZE - (box[3] - box[1])) / 2 - box[1]
    draw.text((x, y), text, font=font, fill=(255, 255, 255, 255))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    base.save(OUT)
    print("wrote", OUT, base.size)


if __name__ == "__main__":
    main()
