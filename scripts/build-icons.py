"""Build application icons from the approved freeRGB Ring export."""
from pathlib import Path
from PIL import Image, ImageOps

root = Path(__file__).resolve().parents[1]
destination = root / "assets" / "icons"
destination.mkdir(parents=True, exist_ok=True)
source = Image.open(root / "assets" / "brand" / "ring-dark.png").convert("RGBA")
square = ImageOps.pad(source, (512, 512), method=Image.Resampling.LANCZOS,
                      color=(23, 23, 23, 255))
square.save(destination / "freergb.png")
square.save(destination / "freergb.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
for size in (16, 24, 32, 48, 64, 128, 256):
    square.resize((size, size), Image.Resampling.LANCZOS).save(
        destination / f"freergb-{size}.png"
    )

