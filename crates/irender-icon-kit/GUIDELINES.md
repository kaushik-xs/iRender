# Icon guidelines

Every icon in this kit is drawn from scratch to the rules below, so icons from different packs sit together as one family. `tools/icons.py lint` checks the measurable rules; the rest are checked by eye on a contact sheet (`tools/icons.py sheet`).

## Canvas

- **24 × 24 grid**, `viewBox="0 0 24 24"`, drawn at 1 unit = 1 px.
- **Padding: 2 px on every side.** The live area is the 20 × 20 square from 2 to 22, and stroke *centrelines* stay within **3 … 21**, so the 1.4 px stroke's edges sit 0.3 px inside it. One exception: shapes that read as wide (vehicles, trays, conveyors) may run their centreline to **2 … 22** horizontally, never vertically.
- **Optical centre.** Centre the icon's visual weight, not its bounding box: a triangle sits a little low, a truck sits on the baseline.

## Keylines

Base shapes give icons the same visual size. Fit the main form to the closest one:

| keyline | centreline geometry |
|---|---|
| circle | `cx 12 cy 12 r 9` |
| square | `x 3 y 3 w 18 h 18` (radius 2) |
| portrait rectangle | `x 5 y 3 w 14 h 18` |
| landscape rectangle | `x 3 y 5 w 18 h 14` |
| baseline | objects that stand on the ground sit on `y 20` (wheels' centrelines on `y 18`, radius 2) |

Smaller details are drawn to the same grid. Don't scale a big icon down to make a detail.

## Stroke

- **1.4 px stroke, outline style.** `fill="none" stroke="currentColor" stroke-width="1.4"`, set once on the root; shapes never set their own stroke or fill. Documents can't change it: the spacing below is designed around the drawing, not the line.
- **Round caps and round joins** (`stroke-linecap="round" stroke-linejoin="round"`).
- **No fills.** A dot is a small circle (radius 0.5 or 0.7 draws a solid dot; radius 1 a ring with a pinhole that reads as a dot), or a zero-length line (a round 1.4 px dot).
- **Even stroke weight.** No doubled strokes; an inner detail is the same 1.4 px as the outline.

## Geometry

- **Coordinates on the half-pixel grid** (`x.0` or `x.5`); prefer whole numbers. Circle centres, radii and rectangle corners likewise.
- **Corner radius:** 2 for outer rectangles of 10 px or more, 1 for smaller ones and inner panels, 0 only where a sharp corner means something (a document's folded corner, a crate's frame).
- **Clear space:** parallel strokes' centrelines **at least 4 apart** (2.6 px of air with the 1.4 px stroke); 3 apart only when a detail cannot fit otherwise. The spacing is measured centreline to centreline so the drawings keep their proportions.
- **Angles:** 0°, 45° and 90° first, then 30° / 60°. Curves are arcs and circles, not freehand béziers, unless the subject is organic (leaves, hands, flames).
- **At most about 8 shapes.** If it needs more, simplify the metaphor.

## Perspective and metaphor

- **Flat, front or side view.** No perspective. 3-D objects whose shape is the point (box, container, pallet) use the same simple three-quarter form throughout the kit: the parcel in `logistics/package` is the template.
- **One idea per icon.** A modifier (plus, check, x, arrow, clock) goes in the **bottom-right**, in a 8 × 8 area from 13 to 21, with a 2 px gap cut out of the base shape around it.
- **Real objects, generic forms.** No brands, logos, trademarks, text or letters (except where the letter *is* the subject, like a currency sign). No people's faces beyond a head circle.
- **Direction:** things that move face right; time and progress go left to right and bottom to top.

## Elements and markup

- Allowed: `path`, `circle`, `ellipse`, `rect`, `line`, `polyline`, `polygon`. No `g`, `transform`, `style`, `class`, `id`, `text`, gradients, masks, clip paths or `<title>`; the component adds the accessible name.
- One element per line, attributes in drawing order (`x y width height rx`, `cx cy r`, `x1 y1 x2 y2`, `d`).
- Paths use absolute commands where it keeps them readable; numbers have no trailing zeros (`3`, `3.5`, not `3.0`).

## Filled style

Every icon also has a filled version, for selected states, small sizes and emphasis. It is drawn from its outline twin, not from scratch, so the two swap without the icon jumping.

- **Same centrelines.** Shapes are drawn on the outline icon's centrelines and are both filled and stroked (the root sets `fill="currentColor"` and the same 1.4 px round stroke). The silhouette's outer edge therefore matches the outline icon's outer edge exactly, and every corner is softened by the stroke.
- **Solid bodies, cut-out details.** The main closed form becomes solid. Details that were lines *inside* it become cut-outs: subpaths of the same `<path>` with `fill-rule="evenodd"`, or gaps between separate solid parts. A cut-out shrinks by 0.7 px on every side (the stroke covers its edge), so draw it 1.4 px larger than it should look; a slot drawn 4 px wide looks 2.6 px wide.
- **Clear space:** cut-outs and gaps between solid parts are drawn **at least 3.5 apart**, centreline to centreline (they look 2.1 px wide); 4 is better.
- **Lines stay lines.** Parts that are lines outside a solid body (a handle, an arrow, a modifier's tick, ring binders) stay strokes. `<polyline>` and open `<path>`s that must not fill set `fill="none"`. `<line>` never fills.
- **Allowed extras:** on shapes, only `fill="none"` and `fill-rule="evenodd"`. Nothing else changes from the outline rules: same live area, grid, keylines, element limits.
- **Simplify where needed.** A detail that can't survive as a cut-out at 16 px is dropped rather than squeezed; the silhouette carries the meaning.
- **Line-only icons** (arrows, chevrons, check, close, plus, minus, menu, …) have nothing to fill. They aren't redrawn: list them in the pack's `filled/_same.txt`, one name per line, and the outline drawing is used for both styles. Every icon must have either a filled file or a `_same.txt` entry.
- Files: `icons/<pack>/filled/<name>.svg`, with the root exactly:

  ```svg
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
  ```

## Naming

- **kebab-case, noun first, then the modifier:** `truck-loading`, `package-check`, `cart-add`.
- Shared suffixes: `-plus`, `-minus`, `-check`, `-x`, `-off` (disabled, drawn with a slash from top-left to bottom-right), `-open`. Never `-filled`: the filled style is a variant of the same name (see above).
- A name means the same thing in every pack. A pack may draw a subject the core pack has when the domain needs its own version (a `hard-hat` in construction and in safety), and the same name may appear in two packs only for the same subject.

## Files

- One icon per file: `icons/<pack>/<name>.svg`, with the root exactly:

  ```svg
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
  ```

- Each pack has `icons/<pack>/_pack.txt`: its title on the first line, a one-line description on the second.
- `tools/icons.py lint` must pass before an icon is merged, and the icon must be checked on a contact sheet at 24 px and 16 px, in light and dark.
