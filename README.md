# FigCraft Plus

**Add-ons for [FigCraft](https://github.com/storytold/figcraft), applied to each official release
and built for Windows.** They only add what FigCraft doesn't have yet: when an official release
ships a feature of its own, it is removed here.

| | |
|---|---|
| **FigCraft Plus** | **3** |
| **Built on** | FigCraft **v0.12.1** (the latest official release) |
| **Files** | `.figcraft` files stay fully compatible with the official app, both ways |

> ### 🗺 Roadmap — what's next
>
> **Next, after FigCraft 0.13 — Plus 4, design system ↔ code:** export variables as CSS
> (`oklch()`, `rem`, relative colours) · import CSS custom properties as variables · remember
> missing-font replacements · edit several text styles at once.
>
> **Then — Plus 5, layout:** sizes in % of the parent · auto layout direction from a variable ·
> align to a key object.
>
> **[See the full roadmap →](ROADMAP.md)** · **[Ask for a feature →](../../issues/new?template=feature_request.yml)**
> — the most 👍-ed requests move up.

This repository holds only the additions. A GitHub Actions workflow takes every new FigCraft
release, applies them, runs the tests and publishes a ready-to-run Windows build under
**[Releases](../../releases)** — usually within a day of the FigCraft release.

## Contents

- [Roadmap](ROADMAP.md)
- [Why FigCraft Plus](#why-figcraft-plus)
- [Download](#download)
- [Requested in FigCraft's issues, made here](#requested-in-figcrafts-issues-made-here)
- [Now in FigCraft itself (removed from Plus)](#now-in-figcraft-itself-removed-from-plus)
- [Add-ons](#add-ons)
  - [OKLCH, done the oklch.com way](#oklch-done-the-oklchcom-way)
  - [Derived colours and calculated variables](#derived-colours-and-calculated-variables)
  - [rem units](#rem-units)
  - [Print units: mm, cm, in and pt in every field](#print-units-mm-cm-in-and-pt-in-every-field)
  - [CMYK and Copy as in the colour picker](#cmyk-and-copy-as-in-the-colour-picker)
  - [Contrast check: WCAG and APCA](#contrast-check-wcag-and-apca)
  - [A better picker](#a-better-picker)
  - [A picker that stays with what you're editing](#a-picker-that-stays-with-what-youre-editing)
  - [Colours driven by number variables](#colours-driven-by-number-variables)
  - [Variables in text: weight, paragraph spacing and indent](#variables-in-text-weight-paragraph-spacing-and-indent)
  - [Your own fonts, travelling with the app](#your-own-fonts-travelling-with-the-app)
  - [Styles: create and edit them in a dialog](#styles-create-and-edit-them-in-a-dialog)
  - [Style groups in Local styles](#style-groups-in-local-styles)
  - [Variable details & scope, one click away](#variable-details--scope-one-click-away)
  - [A richer variable picker](#a-richer-variable-picker)
  - [Reorder variables: drag & drop and sorting](#reorder-variables-drag--drop-and-sorting)
- [How it works](#how-it-works)
  - [Run a build by hand](#run-a-build-by-hand)
  - [Build locally](#build-locally)
  - [When FigCraft adds a feature of its own](#when-figcraft-adds-a-feature-of-its-own)
  - [Add a feature](#add-a-feature)
- [License](#license)

## Why FigCraft Plus

FigCraft Plus is a community effort to make FigCraft more comfortable to work in — above all for
people building design systems, working with CSS and with colour in OKLCH. It adds, on top of each
official release, what users ask for and what we found missing in daily work, and it steps aside
as soon as FigCraft ships its own version: the goal is a better FigCraft, not a different one.

**★ Only in FigCraft Plus (for now)** — FigCraft has nothing like it yet.
**▲ Better in FigCraft Plus** — FigCraft has a basic version; Plus goes further.

| Feature | In FigCraft v0.12.1 | In FigCraft Plus | |
|---|---|---|---|
| [OKLCH, done the oklch.com way](#oklch-done-the-oklchcom-way) | FigCraft 0.12 has OKLCH fields: lightness in percent, sRGB only, and the HSB square and rainbow stay HSB. | The whole picker works in OKLCH (an OKLCH plane with the exact gamut edge, an OKLCH hue strip), values match oklch.com and CSS, and Display P3 files are read in P3. | ▲ better |
| [Derived colours and calculated variables](#derived-colours-and-calculated-variables) | A variable can be an *alias* of another — an exact copy, nothing more. | A colour can be *made from* another (transparency, lighter / darker, its complement, a mix of two), and a number from other numbers, all kept in step in every mode. | ★ only in Plus |
| [rem units](#rem-units) | Sizes are typed in px only; Dev Mode converts px → rem with a fixed 16. | `rem` in every field, a root size per file, an optional rem display in Typography, and Dev Mode uses the file's root size. | ★ only in Plus |
| [Print units: mm, cm, in and pt in every field](#print-units-mm-cm-in-and-pt-in-every-field) | Number fields read px; mm, cm and in exist only in the PDF export's print settings. | Every number field takes `mm`, `cm`, `in` and `pt` — and mixes them (`10mm + 2`). | ★ only in Plus |
| [CMYK and Copy as in the colour picker](#cmyk-and-copy-as-in-the-colour-picker) | The picker offers Hex, RGB, HSL, HSB, LCH, OKLCH and CSS; CMYK exists only as a PDF export option. | **CMYK** fields in the picker, and **Copy as** — the colour in any format, one click. | ★ only in Plus |
| [Contrast check: WCAG and APCA](#contrast-check-wcag-and-apca) | No contrast check in the interface. | Under the picker's fields: the WCAG 2 ratio with its level, and the APCA Lc, against what the colour sits on. | ★ only in Plus |
| [A better picker](#a-better-picker) | Fields without captions, one fixed width. | A caption over every field, a width that fits the model, and a draggable edge. | ▲ better |
| [A picker that stays with what you're editing](#a-picker-that-stays-with-what-youre-editing) | The picker opens at a fixed place on the right and closes when the selection changes. | It opens beside the swatch you clicked, stays on top, and moves to the same slot of the newly selected layer, so you keep editing without reopening it. | ▲ better |
| [Colours driven by number variables](#colours-driven-by-number-variables) | A colour can be bound to a colour variable as a whole. | Each channel (L, C, H, R, G, B, H, S, alpha…) can follow its own number variable: one `brand/hue` restyles every colour built on it. | ★ only in Plus |
| [Variables in text: weight, paragraph spacing and indent](#variables-in-text-weight-paragraph-spacing-and-indent) | Font family and font style take string variables. | The weight takes a number variable too (400, 700), and paragraph spacing and indent take number variables. | ▲ better |
| [Your own fonts, travelling with the app](#your-own-fonts-travelling-with-the-app) | Fonts come from the system (and Adobe Fonts / font managers on macOS). | A `fonts` folder next to `figcraft.exe` — your fonts travel on the USB drive — with *Add fonts…*, an *In this file* section and *Added* tags. | ★ only in Plus |
| [Styles: create and edit them in a dialog](#styles-create-and-edit-them-in-a-dialog) | Styles are created from the selection and edited in FigCraft's style popover. | A **+** on Local styles creates any kind of style from scratch in a dialog with a live preview, with variables in effect, grid and colour fields too. | ▲ better |
| [Style groups in Local styles](#style-groups-in-local-styles) | Local styles are grouped by the folders in their names. | Real nested groups: Ctrl+G, New group, drag styles and groups, collapse, rename, ungroup. | ▲ better |
| [Variable details & scope, one click away](#variable-details--scope-one-click-away) | Right-click a variable ▸ Edit variable. | An *Edit variable* button on every row, opening beside the editor and following it. | ▲ better |
| [A richer variable picker](#a-richer-variable-picker) | A plain list of the variables that fit. | One card everywhere: search, library filter, collections and groups, swatches and values, arrow keys and Enter. | ▲ better |
| [Reorder variables: drag & drop and sorting](#reorder-variables-drag--drop-and-sorting) | Modes can be reordered; variables keep the order they were made in. | Drag variables, groups and collections, multi-select, Alt+↑/↓, and natural or by-value sorting — also as commands for scripts and MCP. | ★ only in Plus |

Everything stays a normal `.figcraft` file: the official app opens it and sees the last computed
colours and sizes; formulas and settings are kept as text it ignores.

## Download

1. Open **[Releases](../../releases)** and take the latest
   `figcraft-plus-<n>-figcraft-<version>-windows-x64-portable.zip`.
2. Unzip it anywhere (a USB drive works) and run `figcraft.exe`.
3. The build isn't code-signed: if Windows SmartScreen warns, choose **More info → Run anyway**.

To hear about new builds, use **Watch → Custom → Releases** on this page.

The build is FigCraft from the same release tag, built the same way (release profile, static C
runtime, embedded fonts, `figcraft-cli.exe` included), plus the add-ons below.

## Requested in FigCraft's issues, made here

Features FigCraft users asked for in the official tracker that FigCraft itself hasn't taken up:

| Request | In FigCraft Plus |
|---|---|
| [storytold/figcraft#1280 — Variable Referencing](https://github.com/storytold/figcraft/issues/1280) | 2 — [derived colours and calculated variables](#derived-colours-and-calculated-variables) |
| [storytold/figcraft#1325 — REM as a typography input unit, with a configurable root size](https://github.com/storytold/figcraft/issues/1325) | 2 — [rem units](#rem-units) |

## Now in FigCraft itself (removed from Plus)

| Was in Plus | In FigCraft since |
|---|---|
| Spacing field for several layers in a row or column | 0.7 — smart-selection spacing, with handles on the canvas |
| OKLCH in the colour picker's model menu | 0.12 — LCH and OKLCH models |
| Font family and font style bound to string variables | 0.12 |
| Colour variables offered by fill / stroke scope in the style picker | 0.12 |
| Google Fonts download | removed earlier (fonts folder instead) |

## Add-ons

### OKLCH, done the oklch.com way

**▲ Better in FigCraft Plus**

> **FigCraft:** FigCraft 0.12 has OKLCH fields: lightness in percent, sRGB only, and the HSB square and rainbow stay HSB.
>
> **FigCraft Plus:** The whole picker works in OKLCH (an OKLCH plane with the exact gamut edge, an OKLCH hue strip), values match oklch.com and CSS, and Display P3 files are read in P3.

<img src="docs/images/02-oklch-picker.png" width="330" alt="The picker in OKLCH: the OKLCH plane, the OKLCH hue strip and the L, C, H, A fields">

| Field | Meaning | Range |
|---|---|---|
| **L** | lightness | 0–1 |
| **C** | chroma | 0–0.4 |
| **H** | hue | degrees |

- Same units and precision as [oklch.com](https://oklch.com) and CSS `oklch(L C H)`: paste a hex
  copied from FigCraft into oklch.com and you get the same numbers. Fields also take CSS forms
  (`64.56%` for L, `55%` for C = 0.22).
- Values are read and written in the **file's colour profile** (sRGB or Display P3,
  File ▸ Color profile). Colours outside the gamut are mapped in with the CSS Color 4 algorithm
  (chroma reduced, lightness and hue kept).
- The whole picker works in OKLCH: the HSB square becomes an **OKLCH plane** (chroma across,
  lightness down, at the current hue) with the exact gamut edge — and, in a Display P3 file, the
  sRGB edge dashed — and the rainbow becomes an **OKLCH hue strip** at the current lightness and
  chroma, so dragging the hue keeps the lightness.
- Verified against the [coloraide](https://github.com/facelessuser/coloraide) reference
  implementation (CSS Color 4) on 6,000 colours: largest difference 0.000001.

### Derived colours and calculated variables

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** A variable can be an *alias* of another — an exact copy, nothing more.
>
> **FigCraft Plus:** A colour can be *made from* another (transparency, lighter / darker, its complement, a mix of two), and a number from other numbers, all kept in step in every mode.

*Requested in [storytold/figcraft#1280](https://github.com/storytold/figcraft/issues/1280).*

A colour can be made from another one, in CSS's own syntax, and stays in step with it — in every
mode (Light / Dark), and down a chain (a hover from a faded colour from the brand colour):

| Typed | Gives |
|---|---|
| `oklch(from {Brand/Primary} l c h / 50%)` | the brand colour, half transparent |
| `oklch(from {Brand/Primary} calc(l - 0.08) c h)` | a darker hover |
| `oklch(from {Brand/Primary} l c calc(h + 180))` | its complement |
| `color-mix(in oklch, {Brand/Primary} 20%, {Surface})` | a tint of the surface |
| `hsl(from #3A81F6 h s calc(l * {Shade}))` | a fixed colour, shaded by a number variable |

- **Type it into the colour picker's CSS field** (the model menu's last entry) — for a colour
  variable, a style's paint or a layer's fill, stroke, gradient stop or effect. The picker shows
  the formula under the fields, with a button that detaches it (the colour stays as it is).
- `from` takes a colour variable `{name}` or a hex colour; the model's keywords (`l c h`, `h s l`,
  `h s b`, `r g b`, `alpha`) are the base colour's channels, in the picker's units, and work in
  `calc()` with numbers and number variables. Without `/ alpha` a derived colour keeps its base's
  alpha.
- `color-mix` mixes in `oklch`, `hsl`, `hsb` or `srgb`, hues the shorter way round; editing a mixed
  colour by hand detaches it.

**Number variables from other number variables:** a line `calc({Space/Base} * 2)` in a number
variable's description keeps its value at twice `Space/Base`, in every mode. Editing the value by
hand detaches it; variables that read each other in a loop are left as they are.

The formulas live in the descriptions (and layer data) as text, so the official FigCraft opens
the files and keeps the last computed values.

### rem units

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** Sizes are typed in px only; Dev Mode converts px → rem with a fixed 16.
>
> **FigCraft Plus:** A **px / rem** switch and a root size per file (the gear in Typography), defaults in Preferences, `rem` in every field, and Dev Mode uses the file's root size.

*Requested in [storytold/figcraft#1325](https://github.com/storytold/figcraft/issues/1325).*

<img src="docs/images/30-typography-settings.png" width="620" alt="The gear in the Typography header opens Typography settings: Unit px or rem, and Root size; the font size shows in rem">

**Where to set it**

| What | Where |
|---|---|
| This file's unit (px or rem) and root size | Select a text layer ▸ **Typography** ▸ the **gear** at the right of the header |
| The unit and root size for new files | **Preferences ▸ FigCraft Plus** ▸ *Typography unit*, *Default root size* |

- With **rem**, font size, line height, letter spacing and paragraph spacing show in rem
  (`1.375rem`); a bare number typed there is rem too, and `22px` still means px.
- **Every number field takes `rem`** whatever the unit: `1.5rem` is 1.5 × the root size, and
  arithmetic works (`2rem + 4`).
- Values are stored in px, as FigCraft stores them; the unit and root size are kept in the file as
  text the official FigCraft ignores, so it opens the file as before.
- Dev Mode's px → rem code uses the file's root size instead of a fixed 16.

### Print units: mm, cm, in and pt in every field

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** Number fields read px; mm, cm and in exist only in the PDF export's print settings.
>
> **FigCraft Plus:** Every number field takes `mm`, `cm`, `in` and `pt` — and mixes them (`10mm + 2`).

<img src="docs/images/31-print-units.png" width="420" alt="210mm typed into a frame's width field">

- **Just type the unit** in the field — `210mm`, `5.5cm`, `1in`, `12pt` — and press Enter; it is
  converted to px. Nothing to switch on.
- A px is a point (1/72 in), as FigCraft's own print export reads it: a frame of `210mm` × `297mm`
  exports as an exact A4 page, `85mm` × `55mm` as a business card.
- Works in any field that takes a number: size, position, radius, stroke, spacing, guides…
- Values are stored in px, as always.

### CMYK and Copy as in the colour picker

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** The picker offers Hex, RGB, HSL, HSB, LCH, OKLCH and CSS; CMYK exists only as a PDF export option.
>
> **FigCraft Plus:** **CMYK** fields in the picker, and **Copy as** — the colour in any format, one click.

<img src="docs/images/32-cmyk-contrast.png" width="330" alt="The colour picker in CMYK, with Copy as and the contrast line under the fields">

- **CMYK** in the model menu: C, M, Y and K in percent. They are converted exactly the way
  FigCraft's CMYK PDF export converts (`K = 1 − max(R, G, B)`), so what you see is what the PDF
  holds. (It's the profile-free formula: proof against your print shop's profile for production.)
- Want the picker to always open in CMYK (or OKLCH)? **Preferences ▸ FigCraft Plus ▸ Colour picker
  opens in**.
- **Copy as** under the fields: Hex, `rgb()`, `hsl()`, HSB, `lch()`, `oklch()` (oklch.com
  precision), `device-cmyk()` or CSS — a converter between colour spaces.

### Contrast check: WCAG and APCA

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** No contrast check in the interface.
>
> **FigCraft Plus:** Under the picker's fields: the WCAG 2 ratio with its level, and the APCA Lc, against what the colour sits on.

- The backdrop is found for you: the nearest frame with a solid fill under the layer, else the
  page (hover the line to see which colour). Transparent colours are blended over it first.
- WCAG levels for normal text: **AAA** ≥ 7, **AA** ≥ 4.5, **AA Large** ≥ 3; below 4.5 the line is
  highlighted. APCA Lc follows APCA-W3 (black on white ≈ 106, white on black ≈ −108).

### FigCraft Plus preferences

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** The colour picker always opens in Hex; there is no typography unit to choose.
>
> **FigCraft Plus:** **Preferences ▸ FigCraft Plus**: the typography unit and root size for new files, and the model the colour picker opens in.

<img src="docs/images/33-preferences.png" width="460" alt="Preferences with the FigCraft Plus section: Typography unit, Default root size, Colour picker opens in">

| Setting | What it does |
|---|---|
| **Typography unit** — px or rem | The unit files use until they set their own (Typography ▸ gear) |
| **Default root size** | 1rem in files that don't set their own |
| **Colour picker opens in** — Hex, RGB, HSL, HSB, LCH, OKLCH, CMYK or CSS | The model the picker shows when it opens |

They are saved in `figcraft-plus.json` next to `figcraft.exe`, so a copy on a USB drive keeps them.

### A better picker

**▲ Better in FigCraft Plus**

> **FigCraft:** Fields without captions, one fixed width.
>
> **FigCraft Plus:** A caption over every field, a width that fits the model, and a draggable edge.

- **Captions over every field** in every model (R G B, H S L, H S B, L C H, A).
- **Width adapts** to the chosen model; drag the picker's **left edge** to make it wider
  (double-click the edge to reset).

### A picker that stays with what you're editing

**▲ Better in FigCraft Plus**

> **FigCraft:** The picker opens at a fixed place on the right and closes when the selection changes.
>
> **FigCraft Plus:** It opens beside the swatch you clicked, stays on top, and moves to the same slot of the newly selected layer, so you keep editing without reopening it.

<p>
<img src="docs/images/21-picker-beside-fill.png" width="420" alt="The picker beside the Design panel, at the fill's height">
<img src="docs/images/20-picker-beside-variable.png" width="450" alt="The picker beside a colour variable's value, above the Variables editor">
</p>

- **It opens beside the control that opened it**: left of the Design panel at the fill's (stroke's,
  effect's…) height; beside a colour variable's value in the Variables editor; beside a style's
  paint. When there's no room it opens on the other side, and it always stays inside the window.
- **It stays on top** of every panel and window, the Variables editor included.
- **It follows its control** when a panel scrolls, the window is resized or the Variables editor
  is moved.
- **Select another layer** (on the canvas or in Layers) and it switches to the same slot of that
  layer — Fill 1 stays Fill 1. If the layer has no such slot, or nothing is selected, it closes.
- **Click empty canvas** (or the same layer again) and it closes; the click still does its usual
  job (an empty spot deselects). Dragging on the canvas (moving layers, gradient handles) leaves
  it open.

### Colours driven by number variables

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** A colour can be bound to a colour variable as a whole.
>
> **FigCraft Plus:** Each channel (L, C, H, R, G, B, H, S, alpha…) can follow its own number variable: one `brand/hue` restyles every colour built on it.

<p>
<img src="docs/images/05-bound-hue-280.png" width="440" alt="H bound to brand/hue = 280: a purple card">
<img src="docs/images/06-bound-hue-130.png" width="440" alt="The same card after brand/hue is set to 130: green">
</p>

*Left: H is bound to `brand/hue` = 280. Right: the variable set to 130 — the card follows.*

Every channel of a colour can be bound to a number variable, right in the picker — just like
FigCraft's own number fields. Hover a field in the **RGB**, **HSL**, **HSB** or **OKLCH** row
(R G B, H S L, H S B, L C H, and alpha **A**): the variable button appears at its right end. Pick
a number variable and the field shows its name; the detach button turns it back into a number.

Bind H to `brand/hue = 280`, change the variable to 130, and every colour linked to it follows —
layers, styles and variables — in one undo step.

**How to bind a channel**

1. Open the picker and choose RGB, HSL, HSB or OKLCH. Hover a field: the variable button appears.

   <img src="docs/images/03-variable-button.png" width="360" alt="The variable button on the H field">

2. Click it and pick a number variable (grouped by collection).

   <img src="docs/images/04-variable-list.png" width="400" alt="The number variable list">

3. The field now shows the variable's name. Alpha binds the same way — here H follows
   `brand/hue` and A follows `brand/fade` = 60%.

   <img src="docs/images/07-hue-and-alpha-bound.png" width="560" alt="H and alpha both bound">

4. To unbind, hover the name and click the detach button at its right; the field gets its
   current value back.

Switching to another model shows which channels are linked elsewhere; binding in the current
model replaces those links.

<img src="docs/images/08-linked-in-other-model.png" width="360" alt="The HSL row noting links made in OKLCH">

- Works wherever the picker edits a colour: layer fills and strokes, **gradient stops**, effect
  colours (shadows, noise), the **page background**, **colour styles** and **colour variables**.
  (Not yet: a range of characters inside a text layer, plugin colour fields.)
- Each model binds in its own terms: H in OKLCH is OKLCH hue, H in HSL is HSL hue. A colour is
  linked in one model at a time; the picker notes links made in another model, and binding in the
  current one replaces them.
- Units are the fields': OKLCH L 0–1 (a variable above 1 is read as a percentage), C 0–0.4,
  H degrees; HSL / HSB H degrees, S L B 0–100; RGB 0–255; alpha 0–100.
- Variables are read in the modes that apply where the colour is: a layer's frames (switch a
  frame to Dark and its layers re-read Dark values), a colour variable's own collection mode;
  styles use the default modes.
- Editing the colour itself (hex, square, eyedropper) updates the unbound channels; bound ones
  stay bound.
- The links are stored as text — a layer's data `figcraftplus.colors`, or a line in a style's or
  colour variable's description, e.g. `fills/0 oklch(0.62 0.19 {brand/hue} / {fade})`. Variables
  are referred to by name: rename one → update the name in the line too. The official FigCraft
  opens such files fine and keeps the last computed colours.

### Variables in text: weight, paragraph spacing and indent

**▲ Better in FigCraft Plus**

> **FigCraft:** Font family and font style take string variables.
>
> **FigCraft Plus:** The weight takes a number variable too (400, 700), and paragraph spacing and indent take number variables.

<p>
<img src="docs/images/12-text-variables.png" width="560" alt="A text layer with font family, weight and paragraph spacing bound to variables">
<img src="docs/images/13-weight-or-style-list.png" width="300" alt="The font style dropdown's list: number and string variables">
</p>

FigCraft binds the font family and font style to string variables. FigCraft Plus binds the rest of
the Typography section:

| Field | Variable | Example |
|---|---|---|
| **Font style / weight** | a **number** (the weight) as well as a string (the style) | `weight/bold` = 700 |
| **Paragraph spacing** | number | `space/para` = 16 |
| **Paragraph indent** (Typography ▸ …) | number | `space/indent` = 24 |

- *Apply variable* in the font style menu lists **both** kinds: a number sets the weight (the
  nearest style the font has is used), a string sets the style by name. Binding one replaces the
  other in one undo step.
- Variables follow their modes like everywhere else (e.g. a Mobile mode with a smaller paragraph
  spacing), and lists respect each variable's **scopes**.
- While editing a range of characters these fields take plain values (bindings are per layer).

### Your own fonts, travelling with the app

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** Fonts come from the system (and Adobe Fonts / font managers on macOS).
>
> **FigCraft Plus:** A `fonts` folder next to `figcraft.exe` — your fonts travel on the USB drive — with *Add fonts…*, an *In this file* section and *Added* tags.

- Font files (`.ttf`, `.otf`, `.ttc`) in a **`fonts` folder next to `figcraft.exe`** are added at
  start — on a USB drive your fonts travel with the app.
- **Add fonts…** at the top of the font menu copies chosen files into that folder and adds them at
  once; they are tagged *Added* in the list.
- The font menu gains an **In this file** section (the families the open file uses), between
  FigCraft's *Recent* and *All fonts*.

### Styles: create and edit them in a dialog

**▲ Better in FigCraft Plus**

> **FigCraft:** Styles are created from the selection and edited in FigCraft's style popover.
>
> **FigCraft Plus:** A **+** on Local styles creates any kind of style from scratch in a dialog with a live preview, with variables in effect, grid and colour fields too.

<p>
<img src="docs/images/15-create-style-menu.png" width="250" alt="The + menu on Local styles: Text, Color, Effect, Layout grid">
<img src="docs/images/22-style-dialog-text.png" width="380" alt="Create new text style: preview, name, font, weight and size bound to variables">
</p>
<p>
<img src="docs/images/23-style-dialog-effect.png" width="440" alt="Create new effect style with the drop shadow's settings beside it">
<img src="docs/images/24-style-dialog-grid.png" width="440" alt="Create new layout guide style with the columns' settings">
</p>
<img src="docs/images/25-style-dialog-color.png" width="440" alt="Create new color style with the colour picker beside it">

The **Local styles** header (Design panel, nothing selected) has a **+**: choose **Text**,
**Color**, **Effect** or **Layout grid** and the **Create new … style** dialog opens — a preview,
**Name** and **Description**, and **Properties**:

- **Text** — font, style (or weight), size, line height, letter spacing, and under **…**
  decoration and case.
- **Color** — its paints: click the swatch for the full colour picker (solid, gradients, image…,
  OKLCH); hex and opacity in the row; **+** adds a paint.
- **Effect** — a row per effect; click it for its settings beside the dialog: the **type**
  (Drop shadow, Inner shadow, Layer blur, Background blur, Noise, Texture, Glass), position,
  blur, spread, colour, "show behind transparent areas"… **+** adds an effect.
- **Layout grid** — a row per grid; its settings: **Grid / Columns / Rows**, size or count, type
  (Stretch, Left, Center, Right), width, margin, gutter, colour. **+** adds a grid.

Every row has a visibility toggle and **−**. **Create style** keeps the style; the cross (or Esc)
drops it. Click a style in Local styles — or **+** and edit in a layer's style list — to open the same
dialog (**Edit … style**; a style made from a layer opens as **Create new … style**) — changes
apply as you make them, each one undo step.

**Variables in styles.** Fields a style can bind take a variable (the hexagon on hover):

| Style | Fields |
|---|---|
| Text | font family (string), style or weight (string or number, as in the Typography panel), size, line height, letter spacing (numbers) |
| Color | each solid paint's colour |
| Effect | shadow X, Y, blur, spread, colour; blur radius |
| Layout grid | size, count, margin, gutter |

Layers using the style follow the variables in their own modes (a frame set to Mobile shows the
Mobile size).

### Style groups in Local styles

**▲ Better in FigCraft Plus**

> **FigCraft:** Local styles are grouped by the folders in their names.
>
> **FigCraft Plus:** Real nested groups: Ctrl+G, New group, drag styles and groups, collapse, rename, ungroup.

Styles sit in **nested groups** (kept as `/` in their names, so the official FigCraft reads them as
folders). Click to select (Ctrl/Shift-click for several), double-click to edit; **Ctrl+G** or
*Group* puts the selection in a group, **+ ▸ New group** makes an empty one; drag styles and whole
groups between rows, into a group or out to the top; collapse a group with its arrow, double-click
its name to rename it, ungroup it from its context menu.

### Variable details & scope, one click away

**▲ Better in FigCraft Plus**

> **FigCraft:** Right-click a variable ▸ Edit variable.
>
> **FigCraft Plus:** An *Edit variable* button on every row, opening beside the editor and following it.

<img src="docs/images/14-edit-variable.png" width="640" alt="The Edit variable button on a row and the variable's details and scopes">

Hover a row in the Variables editor: the **Edit variable** button (two sliders) appears at the end
of the name. It opens the variable's name, description, code syntax and **scoping** — which
properties pickers offer it for (corner radius, gap, font weight, font size, paragraph spacing,
fills, strokes…). The window opens beside the Variables editor at the variable's row, always above
it, and follows the editor when it moves. Click the button again to close. (Right-click ▸ Edit variable still works.)

### A richer variable picker

**▲ Better in FigCraft Plus**

> **FigCraft:** A plain list of the variables that fit.
>
> **FigCraft Plus:** One card everywhere: search, library filter, collections and groups, swatches and values, arrow keys and Enter.

One picker everywhere a variable is applied (layer properties, styles, aliases, prototype
actions, colour channels): search, a library filter, collections and groups as headers, each
variable's swatch or type and its value; arrow keys and Enter work. A variable with every scope
turned off is offered nowhere but the Variables editor, so lists stay short.

### Reorder variables: drag & drop and sorting

**★ Only in FigCraft Plus (for now)**

> **FigCraft:** Modes can be reordered; variables keep the order they were made in.
>
> **FigCraft Plus:** Drag variables, groups and collections, multi-select, Alt+↑/↓, and natural or by-value sorting — also as commands for scripts and MCP.

Variables, groups and collections keep the order you give them: in the editor, in the file and
in exports. That fixes imports that arrive in the wrong order (`n-xs, n-6xl, n-s, n-m …` or
`1, 10, 2, 3`).

<img src="docs/images/11-drag-several.png" width="560" alt="Two selected variables dragged in the table, with the drop line">

- **Drag a variable** up or down in the table; a blue line shows where it lands. Drop it on
  another group's rows or header to move it into that group (it is renamed `group/name`, e.g.
  `space/gap` → `number/gap`). Variables stay in their collection.
- **Select several**: click, then **Shift**-click for a range or **Ctrl**-click to add or remove
  one; drag any selected row to move them all. **Esc** clears the selection (or cancels a drag).
- **Alt + ↑ / ↓** moves the selected variables one row within their group.
- **Drag a group** by its header in the table or its row in the sidebar (to the top level: drop on
  **All variables**). Subgroups move with it.
- **Drag a collection** in the sidebar.
- **Sort**: right-click a group or a collection in the sidebar.

  <img src="docs/images/09-sort-menu.png" width="560" alt="The group menu with Sort by name and Sort by value">
  <img src="docs/images/10-sorted.png" width="560" alt="The group after Sort by name: n-xs, n-s, n-m, n-l, n-xl, n-2xl … n-6xl">

  - **Sort by name** is natural and knows sizes: `xxs, xs, s, m, l, xl, 2xl … 6xl` and
    `1, 2, 10`.
  - **Sort by value** goes low → high: numbers by value, colours light → dark, then text and
    booleans.
  - Groups keep their places; variables are sorted inside each group.
- Each drag, Alt+arrow or sort is one undo step. Read-only libraries can't be reordered.
- The same operations are commands for scripts, the CLI and MCP: `variable.move`,
  `variable.moveGroup`, `variable.moveCollection`, `variable.sort`.

## How it works

```
overlay/          the additions' own files, copied into FigCraft at the same paths
hooks/            where they connect: "insert after this line" / "replace this text"
scripts/apply.py  copies overlay/ and applies hooks/ to a FigCraft checkout
CUSTOM_VERSION    the FigCraft Plus version (bump after changing the additions)
.github/workflows/build.yml   the daily build
```

Every day at 04:17 UTC the workflow checks FigCraft's latest release. If this repository hasn't
built it yet (or the additions changed), it checks the release out, runs `scripts/apply.py`, runs
the additions' tests and publishes `v<figcraft version>-plus.<CUSTOM_VERSION>` under Releases.

If a FigCraft update moves code a hook relies on, the build stops at **Connect custom features**
with the hook named (e.g. `0001-oklch-colour-picker.json #5 (picker: OKLCH fields): no line
matches …`), GitHub e-mails the owner, and the previous release stays available. Fix the hook's
anchor text — usually one line — and the next build goes through.

### Run a build by hand

**Actions → Build FigCraft + custom → Run workflow**. Optionally give a FigCraft tag (e.g.
`v0.6.0`) or tick **force** to rebuild a version that is already released.

### Build locally

```sh
git clone https://github.com/storytold/figcraft
python3 scripts/apply.py figcraft          # --check lists every hook that no longer fits
cd figcraft && cargo run --release -p figcraft
```

### When FigCraft adds a feature of its own

Remove ours: delete its hook (and its `overlay/` files, or the parts of them it no longer needs),
move it to *Now in FigCraft itself* above and bump `CUSTOM_VERSION`.

### Add a feature

1. Put its code in new files under `overlay/` (same paths as in FigCraft).
2. Connect them with a new `hooks/<next number>-….json` (see `0001` for the format).
3. Bump `CUSTOM_VERSION`. Pushing starts a build.

## License

FigCraft is MIT OR Apache-2.0; these additions are offered under the same terms. Not affiliated
with the ArtCraft team. FigCraft is a trademark of its owners.
