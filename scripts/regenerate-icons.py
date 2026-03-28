#!/usr/bin/env python3
"""Regenerate all Tauri app icons from source-logo.png.

Uses the source logo as-is (white background with copper mark),
adds rounded corners (squircle) and proper Apple HIG sizing.

macOS: 824x824 squircle centered on 1024x1024 transparent canvas.
Windows: Full-bleed squircle.
"""

import subprocess
import tempfile
from pathlib import Path
from PIL import Image, ImageDraw

ICONS_DIR = Path(__file__).parent.parent / "src-tauri" / "icons"
SOURCE = ICONS_DIR / "source-logo.png"

# Apple HIG: 1024x1024 canvas, 824x824 content area, 100px padding each side
CANVAS_SIZE = 1024
CONTENT_SIZE = 824
PADDING = 100

# macOS squircle corner radius (~18% of content size)
CORNER_RADIUS_RATIO = 0.181


def create_rounded_rect_mask(size: int, radius: int) -> Image.Image:
    """Create a smooth rounded rectangle alpha mask.

    Renders at 4x then downscales for anti-aliased edges.
    """
    scale = 4
    big = size * scale
    big_r = radius * scale

    mask = Image.new("L", (big, big), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle([0, 0, big - 1, big - 1], radius=big_r, fill=255)

    mask = mask.resize((size, size), Image.LANCZOS)
    return mask


def apply_squircle(img: Image.Image, size: int) -> Image.Image:
    """Resize source image to target size and apply squircle mask."""
    resized = img.resize((size, size), Image.LANCZOS).convert("RGBA")

    radius = int(size * CORNER_RADIUS_RATIO)
    mask = create_rounded_rect_mask(size, radius)
    resized.putalpha(mask)

    return resized


def create_macos_master(source: Image.Image) -> Image.Image:
    """824x824 squircle on 1024x1024 transparent canvas (Apple HIG)."""
    content = apply_squircle(source, CONTENT_SIZE)
    canvas = Image.new("RGBA", (CANVAS_SIZE, CANVAS_SIZE), (0, 0, 0, 0))
    canvas.paste(content, (PADDING, PADDING), content)
    return canvas


def create_windows_master(source: Image.Image) -> Image.Image:
    """Full-bleed squircle, no padding."""
    return apply_squircle(source, CANVAS_SIZE)


def generate_png(master: Image.Image, filename: str, size: int) -> None:
    resized = master.resize((size, size), Image.LANCZOS)
    resized.save(ICONS_DIR / filename, "PNG")
    print(f"  {filename} ({size}x{size})")


def generate_ico(master: Image.Image) -> None:
    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = [master.resize((s, s), Image.LANCZOS) for s in sizes]
    images[0].save(
        ICONS_DIR / "icon.ico",
        format="ICO",
        sizes=[(s, s) for s in sizes],
        append_images=images[1:],
    )
    print(f"  icon.ico (sizes: {sizes})")


def generate_icns(master: Image.Image) -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        iconset_dir = Path(tmpdir) / "icon.iconset"
        iconset_dir.mkdir()

        icns_sizes = [
            ("icon_16x16.png", 16),
            ("icon_16x16@2x.png", 32),
            ("icon_32x32.png", 32),
            ("icon_32x32@2x.png", 64),
            ("icon_128x128.png", 128),
            ("icon_128x128@2x.png", 256),
            ("icon_256x256.png", 256),
            ("icon_256x256@2x.png", 512),
            ("icon_512x512.png", 512),
            ("icon_512x512@2x.png", 1024),
        ]

        for name, size in icns_sizes:
            resized = master.resize((size, size), Image.LANCZOS)
            resized.save(iconset_dir / name, "PNG")

        output = ICONS_DIR / "icon.icns"
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset_dir), "-o", str(output)],
            check=True,
        )
        print(f"  icon.icns (via iconutil)")


def main() -> None:
    source = Image.open(SOURCE)
    print(f"Source: {source.size[0]}x{source.size[1]}")

    print("\nGenerating macOS master (824x824 squircle + 100px padding)...")
    macos_master = create_macos_master(source)

    print("Generating Windows master (full-bleed squircle)...")
    windows_master = create_windows_master(source)

    print("\nSaving macOS PNGs...")
    generate_png(macos_master, "icon.png", 1024)
    generate_png(macos_master, "128x128@2x.png", 256)
    generate_png(macos_master, "128x128.png", 128)
    generate_png(macos_master, "64x64.png", 64)
    generate_png(macos_master, "32x32.png", 32)

    print("\nSaving Windows Store logos...")
    generate_png(windows_master, "Square310x310Logo.png", 310)
    generate_png(windows_master, "Square284x284Logo.png", 284)
    generate_png(windows_master, "Square150x150Logo.png", 150)
    generate_png(windows_master, "Square142x142Logo.png", 142)
    generate_png(windows_master, "Square107x107Logo.png", 107)
    generate_png(windows_master, "Square89x89Logo.png", 89)
    generate_png(windows_master, "Square71x71Logo.png", 71)
    generate_png(windows_master, "Square44x44Logo.png", 44)
    generate_png(windows_master, "Square30x30Logo.png", 30)
    generate_png(windows_master, "StoreLogo.png", 50)

    print("\nGenerating ICO...")
    generate_ico(windows_master)

    print("Generating ICNS...")
    generate_icns(macos_master)

    print("\nDone!")


if __name__ == "__main__":
    main()
