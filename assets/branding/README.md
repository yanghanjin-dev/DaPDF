# DaPDF icon v1

## Active application icon

- Selected design: paper airplane and PDF document, matching the user's selected attachment (alternative 09, `alternatives/dapdf-09-paper-plane.png`).
- `dapdf-icon.png`: unchanged copy of selected PNG artwork.
- `dapdf.ico`: Windows application icon with 16, 24, 32, 48, 64, 128 and 256 px images.
- `build.rs` embeds the ICO as Windows resources via `build/icon_resource.rs`.
- The main window loads group icon resource 1 for its window and taskbar icon.
- The v1 eye design below is retained as an unused earlier draft.

- `dapdf-icon-v1.png`: original generated artwork with transparency.
- `dapdf-icon-v1.ico`: Windows icon container (16, 24, 32, 48, 64, 128, 256 px).
- Created using the built-in image generation tool. PNG-to-ICO format conversion uses Pillow.
- Concept: a folded PDF document, an eye for "view all PDFs", and open pages for a versatile document workspace. Red/coral tile, white mark and DaPDF wordmark, navy pupil.
- This earlier v1 asset is not used by the executable.

## Generation prompt

Use case: logo-brand. Create one original production-ready Windows application icon for DaPDF, meaning "view all PDFs", a PDF viewer expanding into document conversion and editing. Reference the user's two PDF icon moodboards only for general document-icon familiarity; create a distinctive original mark, no Adobe Acrobat ribbon or other existing brand marks. Square 1024x1024 composition, transparent outside the icon. A bold red/coral rounded-square app tile with a crisp white folded-corner document silhouette. Integrate a strong simple stylized eye into the document, communicating seeing every PDF; the eye's flowing upper/lower curves subtly suggest two open document pages and an inclusive encompassing shape. A restrained dark navy pupil provides focal contrast. Modern premium flat geometric design, optical balance, generous negative space, exceptionally clean edges, minimal layers, no photorealism, no mockup. At the bottom inside the tile use a single short bold clean white sans-serif wordmark spelled exactly "DaPDF", large and readable. No other text, no extra floating symbols, no tiny conversion badges, no multiple concepts or contact sheet. Strong identifiable silhouette readable at 32 pixels. The mark conveys viewing and a versatile document workspace rather than only one export format. Tile fills about 86 percent of the image with even transparent margin; no external shadow, no decorative background.
