#!/usr/bin/env python3
"""Regenerate the standalone gateway assets with POV-Ray 3.7, Pillow and librsvg."""

import argparse
from math import exp
from pathlib import Path
import subprocess

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = Path(__file__).resolve().parent
TYPE = HERE.parents[1] / "shell" / "drawing"


def grade(source):
    """Warm the lit masonry against cool, darker shadows and let the sky carry the menu."""
    image = source.convert("RGB")
    w, h = image.size
    pixels = bytearray(image.tobytes())
    for y in range(h):
        sy = y / h
        top = max(0, 1 - sy / .40) ** 1.9
        horizon = exp(-((sy - .348) / .095) ** 2)
        for x in range(w):
            sx = x / w
            i = (y * w + x) * 3
            r, g, b = pixels[i:i+3]
            luma = r*.22 + g*.69 + b*.09
            dark = max(0, 1 - luma / 150)
            falloff = ((sx - .54) ** 2 * .20 + (sy - .49) ** 2 * .32 + top*.18)
            halo = max(0, sx - .26) * horizon
            for j, (v, shift) in enumerate(((r, -7*dark + 24*halo),
                                            (g, -2*dark + 8*halo),
                                            (b, 7*dark - 9*halo))):
                value = (luma - 76) * 1.15 + 72 + (v - luma) * 1.23 + shift
                pixels[i+j] = max(0, min(255, round(value * (1 - falloff))))
    return Image.frombytes("RGB", image.size, bytes(pixels))


def centred_text(mask, text, font, y, tracking):
    draw = ImageDraw.Draw(mask)
    widths = [font.getlength(ch) for ch in text]
    width = sum(widths) + tracking * (len(text) - 1)
    x = (mask.width - width) / 2
    for ch, advance in zip(text, widths):
        draw.text((x, y), ch, font=font, fill=255, anchor="la")
        x += advance + tracking


def title_mark():
    size = (360 * 3, 69 * 3)
    mask = Image.new("L", size)
    serif = ImageFont.truetype(TYPE / "IBMPlexSerif-Medium.ttf", 110)
    sans = ImageFont.truetype(TYPE / "IBMPlexSansCondensed-Regular.ttf", 25)
    centred_text(mask, "OPEN LORDS II", serif, -9, 5.1)
    strap = Image.new("L", size)
    centred_text(strap, "A CHRONICLE OF CROWNS", sans, 143, 6.6)
    shadow = Image.new("RGBA", size, (10, 12, 17))
    shadow.putalpha(mask.filter(ImageFilter.MaxFilter(11)).filter(ImageFilter.GaussianBlur(8)))
    metal = Image.new("RGBA", size)
    draw = ImageDraw.Draw(metal)
    for y in range(size[1]):
        t = max(0., min(1., (y - 23) / 85))
        color = tuple(round(a + (b-a)*t) for a, b in zip((255, 237, 181), (193, 146, 83)))
        draw.line((0, y, size[0], y), fill=color)
    metal.putalpha(mask)
    shadow.alpha_composite(metal)
    subtitle = Image.new("RGBA", size, (228, 212, 170))
    subtitle.putalpha(strap)
    shadow.alpha_composite(subtitle)
    draw = ImageDraw.Draw(shadow)
    for lo, hi in [(196, 309), (771, 884)]:
        draw.line((lo, 162, hi, 162), fill=(194, 158, 101, 195), width=2)
    return shadow


def buttons():
    w, h = 192 * 3, 24 * 3
    sheet = Image.new("RGBA", (w, 2 * h))
    for active in (False, True):
        plate = Image.new("RGBA", (w, h))
        draw = ImageDraw.Draw(plate)
        shape = [(0, 9), (9, 0), (w-10, 0), (w-1, 9), (w-1, h-10),
                 (w-10, h-1), (9, h-1), (0, h-10)]
        rim = (213, 169, 98, 250) if active else (157, 142, 111, 194)
        draw.polygon(shape, fill=(16, 23, 30, 207), outline=rim, width=3)
        edges = ((66, 52, 35), (29, 30, 33)) if active else ((27, 37, 45), (18, 28, 36))
        for y in range(4, h-4):
            t = (y-4)/(h-9)
            rgb = tuple(round(a + (b-a)*t) for a, b in zip(*edges))
            inset = 4 + max(0, 10-y, y-(h-11))
            draw.line((inset, y, w-1-inset, y), fill=(*rgb, 239 if active else 190))
        if active:
            for x in (35, w-36):
                draw.polygon(((x-6, h//2), (x, h//2-6), (x+6, h//2), (x, h//2+6)),
                             fill=(240, 196, 112, 255))
        sheet.paste(plate, (0, h if active else 0))
    return sheet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--povray", default="povray")
    parser.add_argument("--pov-include", type=Path)
    parser.add_argument("--rsvg-convert", default="rsvg-convert")
    parser.add_argument("--workdir", type=Path, default=HERE.parents[4] / "dist" / "gateway-art")
    args = parser.parse_args()
    args.workdir.mkdir(parents=True, exist_ok=True)
    raw = args.workdir.resolve() / "castle-rays.png"
    svg = args.workdir.resolve() / "shields.png"
    include = [f"+L{args.pov_include.resolve()}"] if args.pov_include else []
    subprocess.run([args.povray, *include, f"+I{HERE / 'gateway.pov'}", f"+O{raw}",
                    "+W1920", "+H1440", "+FN", "+A0.16", "+AM2", "+R3", "-J", "-D", "-V"], check=True)
    subprocess.run([args.rsvg_convert, str(HERE / "shields.svg"), "-o", str(svg)], check=True)
    with Image.open(raw) as source:
        grade(source).save(HERE / "gateway.qoi")
    with Image.open(svg) as source:
        source.convert("RGBA").save(HERE / "shields.qoi")
    title_mark().save(HERE / "gateway-title.qoi")
    buttons().save(HERE / "gateway-buttons.qoi")


if __name__ == "__main__":
    main()
