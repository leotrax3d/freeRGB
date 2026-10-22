# freeRGB brand assets

The segmented RGB Ring is the primary application mark. The lowercase f is its
secondary form. Both share red `#EF4444`, green `#10A37F`, blue `#3B82F6` and a
12-unit stroke on a 128-unit grid.

Source: [Figma brand family](https://www.figma.com/design/xoYLLKkvSQJNG1ViPVVfZW?node-id=36-108).

## Files

- `brand/ring-{dark,light}.svg` and `.png`: primary application mark.
- `brand/monogram-{dark,light}.svg` and `.png`: secondary f mark.
- `brand/{ring,monogram}-tray-{white,dark}.svg` and `.png`: monochrome tray marks.
- `brand/wordmark-{dark,light}.svg` and `.png`: icon with outlined freeRGB lettering.
- `brand/github-banner.svg` and `.png`: 1600 × 480 README banner.
- `brand/github-social-preview.svg` and `.png`: 1280 × 640 social preview artwork.
- `brand/github-avatar.svg` and `.png`: 512 × 512 avatar artwork.
- `brand/loading-ring-monogram.svg`: looping 3.2-second Ring → f → Ring animation.
- `icons/freergb.png`: 512 × 512 application icon.
- `icons/freergb.ico`: Windows icon with 16, 24, 32, 48, 64, 128 and 256 px images.
- `icons/freergb-{size}.png`: matching raster sizes for desktop integrations.

The static SVG marks and GitHub artwork were exported from Figma. PNG versions
were rasterized from those vectors. Wordmarks reuse the exported icon and outlined
lettering. The animated SVG reproduces the Figma geometry and keyframe timing;
it is not a native Figma animation export. Reduced-motion users see the static Ring.

Rebuild application icons with `python scripts/build-icons.py` (Pillow required).
`logo.png` is a compatibility copy of the primary Ring. Historical artwork at the
root of `assets/` is retained; use `assets/brand/` for the current identity.

The README uses the banner directly. Committing social-preview and avatar artwork
does not change GitHub repository settings or the account avatar.
