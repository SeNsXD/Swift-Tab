"""Builds the Swift Tab icons from the two brand assets:
- assets/swift-tab-logo.png (with background) -> assets/nowbar.ico (Resource ID 1: main app .exe logo)
- ui/syro-mark.png (no background) -> assets/nowbar-nobg.ico (Resource ID 2: tray & taskbar) and assets/nowbar-256.png

Run: python assets/make_icon.py   (needs Pillow)"""
import os
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]


def save_ico(src_png: str, dst_ico: str):
    base = Image.open(src_png).convert("RGBA")
    images = [base.resize((n, n), Image.Resampling.LANCZOS) for n in SIZES]
    images[-1].save(dst_ico, sizes=[(n, n) for n in SIZES], append_images=images[:-1])


def main():
    bg_png = os.path.join(HERE, "swift-tab-logo.png")
    nobg_png = os.path.join(ROOT, "ui", "syro-mark.png")
    if os.path.exists(bg_png):
        save_ico(bg_png, os.path.join(HERE, "nowbar.ico"))
    if os.path.exists(nobg_png):
        save_ico(nobg_png, os.path.join(HERE, "nowbar-nobg.ico"))
        Image.open(nobg_png).convert("RGBA").resize((256, 256), Image.Resampling.LANCZOS).save(
            os.path.join(HERE, "nowbar-256.png")
        )
    print("ok")


if __name__ == "__main__":
    main()
