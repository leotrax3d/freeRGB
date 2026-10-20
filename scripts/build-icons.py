"""Convert the existing README logo to application icon formats without redesign."""
from pathlib import Path
from PIL import Image, ImageOps

root = Path(__file__).resolve().parents[1]
destination = root / "assets" / "icons"
destination.mkdir(parents=True, exist_ok=True)
source = Image.open(root / "logo.png").convert("RGBA")
square = ImageOps.pad(source, (256, 256), method=Image.Resampling.LANCZOS,
                      color=(23, 23, 23, 255))
square.save(destination / "freergb.png")
square.save(destination / "freergb.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
