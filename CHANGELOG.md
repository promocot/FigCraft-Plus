# Changelog

Every FigCraft Plus release, newest first. Each one is built on an official FigCraft release
(named in the heading); downloads are under [Releases](../../releases). What comes next is in the
[roadmap](ROADMAP.md).

**🟣 Exclusive** — FigCraft has nothing like it yet. **🔵 Enhanced** — FigCraft has a basic
version; Plus goes further.

## Plus 8 · FigCraft v0.12.1 — 2026-10-30

- 🟣 **Switch units from any field.** A length field names its unit right after its value: click
  it — or right-click any length field, also in px, where fields stay exactly as FigCraft draws
  them — and pick a unit: it becomes the **document's** unit (layout or text), never a single
  property's. *Units settings…* at the end of the list. Font size now works like every other text
  length.
- 🔵 **Popovers close the same way:** their close button, Escape, a click outside and a change of
  selection — the variable list, the style menu, Units, and the colour picker (which still moves to
  the same slot of a newly selected layer). Compact menus stay compact.
- 🟣 **Decimals shown** (0–4) in Preferences ▸ FigCraft Plus. Only what fields show is rounded;
  values keep every digit.

## Plus 7 · FigCraft v0.12.1 — 2026-10-30

- 🟣 **Units everywhere.** Each file picks the unit its fields show: one for **layout** — px,
  rem, mm, cm, in or pt — and one for **text** — px, rem or pt — with its own root size. Position,
  size, min / max, radius, gap and padding, strokes, shadows and blurs, layout grids, vector
  points, the spacing of selected layers, the selection's size on the canvas and the measurements
  all show the layout unit; font size, line height, letter and paragraph spacing, indent and list
  spacing the text unit. Any field still takes any unit typed by hand (`210mm`, `1.5rem`).
  Set it in **Units**: the gear in the Layout or Typography header, or the Units row of the Page
  section; defaults in Preferences ▸ FigCraft Plus.
- 🔵 **One popover style** across the Design panel: the colour picker, the variable list, the
  style menu (now light in the light theme), the Units settings and the style dialogs share one
  card — border, radius, shadow, a header with the title and a close button, a divider under it.

## Plus 6 · FigCraft v0.12.1 — 2026-10-30

- 🔵 **Libraries tab in the colour picker.** Beside *Custom*: this file's and the enabled
  libraries' colour variables, with search, collections and groups. Only variables whose scope
  fits what you edit are listed (a stroke offers stroke colours, a frame's fill frame-fill
  colours, a shadow effect colours). Works for fills, strokes, effect colours, colour styles and
  variable values (as an alias).
- 🟣 **Contrast check sees the real background.** WCAG and APCA are measured against the layers
  painted under the layer — a button on a coloured rectangle is checked against the rectangle,
  not the page. Half-transparent layers are blended; gradients count by their average.
- The variable picker offers, for several selected layers, only variables scoped for all of them.
- README: hero, *Why FigCraft Plus*, animated demos, before / after, compatibility, community,
  FAQ. The build now stops if an add-on would touch FigCraft's file format.

## Plus 5 · FigCraft v0.12.1 — 2026-10-30

- The Typography settings popover (the gear) opens reliably, with a solid background, beside the
  properties panel and always inside the window.
- Screenshots of every new setting in the README.

## Plus 4 · FigCraft v0.12.1 — 2026-10-30

- 🟣 **Typography settings** (the gear in Typography): this file's unit — px or rem — and root
  size, as asked in [storytold/figcraft#1325](https://github.com/storytold/figcraft/issues/1325).
- 🟣 **Preferences ▸ FigCraft Plus:** the typography unit and root size for new files, and the
  model the colour picker opens in (Hex, RGB, HSL, HSB, LCH, OKLCH, CMYK or CSS).

## Plus 3 · FigCraft v0.12.1 — 2026-10-30

- 🟣 **Print units** in every number field: `mm`, `cm`, `in`, `pt`, mixed with arithmetic
  (`210mm` × `297mm` is an exact A4 page).
- 🟣 **CMYK** in the colour picker (converted as FigCraft's CMYK PDF export converts) and
  **Copy as**: the colour as Hex, `rgb()`, `hsl()`, HSB, `lch()`, `oklch()`, `device-cmyk()` or
  CSS.
- 🟣 **Contrast check** under the picker's fields: WCAG 2 ratio and level, APCA Lc.

## Plus 2 · FigCraft v0.12.1 — 2026-10-29

- 🟣 **Derived colours and calculated variables** — `oklch(from {Brand/Primary} …)`,
  `color-mix()`, `calc()` between number variables, kept in step in every mode — as asked in
  [storytold/figcraft#1280](https://github.com/storytold/figcraft/issues/1280).
- 🟣 **rem units** with a root size per file, as asked in
  [storytold/figcraft#1325](https://github.com/storytold/figcraft/issues/1325).

## Plus 1 · FigCraft v0.12.1

The first release of FigCraft Plus on FigCraft 0.12: everything FigCraft 0.12 now does itself was
removed (OKLCH in the model menu, string variables for fonts, scoped colour variables in the style
picker, smart-selection spacing), and the rest carried over:

- 🔵 OKLCH the oklch.com way: an OKLCH plane with the exact gamut edge, an OKLCH hue strip, values
  that match oklch.com and CSS, Display P3 files read in P3.
- 🟣 Colour channels driven by number variables.
- 🔵 A picker that opens beside what you edit and follows the selection; captions on every field
  and a resizable width.
- 🔵 Variables in text: weight, paragraph spacing and indent.
- 🔵 Styles created and edited in a dialog with a live preview; nested style groups.
- 🔵 A richer variable picker; *Edit variable* on every row; 🟣 variable reordering and sorting.
- 🟣 A portable `fonts` folder next to the app.
