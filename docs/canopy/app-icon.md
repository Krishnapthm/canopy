# App icon

Canopy uses the Figma artwork for its macOS bundle, Linux launcher and Windows executable.

To update it, export a square 1024 × 1024 PNG from Figma and replace the three files in `assets/icons/` together. Rebuild the package to include the new artwork.

The macOS bundle and disk image use `Canopy.icns`, which carries standard and Retina sizes. Linux keeps the launcher’s `canopy.svg` path and embeds the PNG in that file. Windows embeds the sizes from `canopy.ico` when it builds the executable.
