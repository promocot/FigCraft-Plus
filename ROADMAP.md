# FigCraft Plus — Roadmap

What we are working on, what comes next, and how to get a feature onto the list.

FigCraft Plus follows the official FigCraft releases. Each time a new FigCraft version comes out,
we first make sure every add-on still works on it, remove what FigCraft now does itself, and then
continue down this list. The work is planned in **stages**, each tied to a FigCraft version: a
stage starts once the add-ons are checked on that version. There are no dates — FigCraft's own
release pace sets them — and the plan is a forecast: requests and what FigCraft ships can move
things.

**Our three specialities:** 🎨 **Advanced color** · 🧩 **Design systems** · 🔁 **Design to code**.
Everything below serves one of them.

**Want something?** [Ask for a feature](../../issues/new?template=feature_request.yml) — and give a
👍 to the requests you'd like too. The most wanted ones move up.

| Stage | When | Theme |
|---|---|---|
| **1** | ✅ done in Plus 9 | Measurement, part 2 · contrast both ways |
| **2** | now, on FigCraft 0.13 | Design system ↔ code |
| **3** | on FigCraft 0.13 | Design System Inspector |
| **4** | after FigCraft 0.14 | Responsive layout |
| **5** | after FigCraft 0.14 | Web typography and colour tokens |
| — | later | Print and export |

---

## ▶ Stage 2 — now, on FigCraft 0.13: design system ↔ code

*FigCraft already exports variables as JSON, CSS (in px), Tailwind and Dart, and imports JSON. Plus
adds what it leaves out.*

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Richer CSS export** — colours as `oklch()`, sizes in `rem` where the document uses rem, derived colours as relative colours (`oklch(from var(--primary) l c h / 50%)`), calculated numbers as `calc()` | Hand developers exactly what the design means, not a flattened copy |
| ⬜ | **Import CSS custom properties as variables** — `--space-m: 1rem`, `oklch()`, `var()` (as an alias), `calc()` (as a calculated variable), `:root` and theme blocks as modes | Bring a site's design system into FigCraft in one step |
| ⬜ | **Edit several text styles at once** — select styles in Local styles, change font, size or line height for all | Restyle a whole type scale in one go |
| ⬜ | **Bind variables to many styles at once** — pick text styles, map their size, line height, weight… to variables by name, preview, apply as one undo step | Hundreds of clicks to move a type scale onto variables become one |
| ⬜ | **Remember missing-font replacements** — "always use Inter for Helvetica" | Open files from others without choosing the same replacements again |

## 🔍 Stage 3 — on FigCraft 0.13: Design System Inspector

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Design System Inspector** — one panel that checks the whole document: colours and sizes not on a variable (with the variable that matches, one click to bind), broken references, circular formulas, unused variables, variables without a scope, duplicate tokens, invalid values. Every finding jumps to its layer or variable; fixes are one undo step | Keep a design system healthy without hunting by hand |

## 📐 Stage 4 — after FigCraft 0.14: responsive layout

*The deepest changes: sizes that follow the parent frame, computed by the layout engine. They come
last because layout is the code most likely to change between FigCraft releases, so they need the
most porting.*

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Sizes in %, vw and vh** — width 50%, height 100vh: relative to the parent frame (which one is always shown), kept in step when it changes; `%`, `vw`, `vh` in every size field | Responsive layouts without recalculating by hand; files stay plain px for the official app |
| ⬜ | **Auto layout from variables** — direction (row / column), Hug / Fill / Fixed and alignment taken from variables, so modes switch them | A card that is a row on Desktop and a column on Mobile — one component, not two |
| ⬜ | **Align to a key object** — select several layers, click one: the others align to it and it stays put | Precise layout without moving the reference |

## 🔤 Stage 5 — after FigCraft 0.14: web typography and colour tokens

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Line height in % from a variable** — `leading/normal = 140%` follows each text's size | Tight / Normal / Relaxed as tokens, as CSS writes them |
| ⬜ | **Gradient tokens** — a gradient style built from colour variables, with stops interpolated in OKLCH; change the palette and every gradient follows | Gradients become part of the design system (no new file format: a style with bound stops) |

## 📦 Later

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Export presets** — a set of sizes, formats and a file-name pattern (`{frame}_{w}x{h}`), exported in one click | Product cards and banners in every size at once |
| ⬜ | **PDF/X for print** — an output intent with an ICC profile on top of FigCraft's print PDF (bleed, marks and CMYK are already FigCraft's) | Files a print shop accepts as they are |
| ⬜ | **Design tokens (DTCG) import and export** | Exchange tokens with other tools in the standard format |

## 💡 Ideas we are considering

Not planned yet — a 👍 on a [feature request](../../issues?q=is%3Aissue+label%3Aenhancement) helps
decide.

- Responsive component states (Desktop / Tablet / Mobile) switched by variable modes
- OKLCH tonal scale generator: 50 … 950 from one colour, as variables that follow it
- OKLCH gamut views in the picker: lightness, chroma and hue slices of the gamut, to see how far a
  colour can go
- A key to hide the selection outline while editing
- Remappable keyboard shortcuts

## Already in FigCraft — use it there

Often asked for, and FigCraft has it already, so Plus doesn't add it: **Paste to replace**
(Edit menu) · **stroke weight per side** · **variable font axes** (Typography) · **bleed, crop
marks and CMYK in the print PDF** · **line height in %** (as a value) · **layout grid styles** ·
**gradient stops bound to colour variables** · **outline view** · **variables export** (JSON, CSS,
Tailwind, Dart) **and import** (JSON) · **px or rem in Dev Mode's code** · since 0.13: **one popover
style** that closes on Escape and a press outside · **the colour picker beside the Design panel** ·
**colour variables with an alpha, and mixes of two** (as variable values) · **a portable mode**
(settings next to the app) · **custom dash patterns** for strokes.

## ✅ Done

Full details in the [changelog](CHANGELOG.md).

| Release | Highlights |
|---|---|
| **Plus 9** | On FigCraft 0.13.1 (FigCraft's own popovers now) · rulers and Dev Mode in the document's unit, the code in px or rem · number variables shown in the unit of what they size · em for line height and letter spacing · contrast both ways |
| **Plus 8** | Switch the document's unit from any length field (click its unit, or right-click) · one way to close every popover · decimals shown, 0–4 |
| **Plus 7** | **Units everywhere:** a layout unit (px, rem, mm, cm, in, pt) and a text unit (px, rem, pt) per file, in every field and on the canvas · one popover style across the Design panel |
| **Plus 6** | Contrast check against what is really under the layer (any shape behind it) · a **Libraries** tab in the colour picker with the colour variables whose scope fits |
| **Plus 5** | The Typography settings popover opens reliably, beside the panel, inside the window · screenshots of every new setting in the README |
| **Plus 4** | Typography settings (the gear): px / rem and root size per file ([FigCraft #1325](https://github.com/storytold/figcraft/issues/1325)) · Preferences ▸ FigCraft Plus: default typography unit and root size, the model the colour picker opens in |
| **Plus 3** | Print units (mm, cm, in, pt) in every field · CMYK and *Copy as* in the colour picker · WCAG / APCA contrast check |
| **Plus 2** | Derived colours and calculated variables ([requested in FigCraft #1280](https://github.com/storytold/figcraft/issues/1280)) · rem units with a root size per file ([requested in FigCraft #1325](https://github.com/storytold/figcraft/issues/1325)) |
| **Plus 1** | OKLCH the oklch.com way · colour channels from number variables · a picker that stays with what you edit · text, style and variable improvements · portable fonts folder |

---

## How a request gets on the roadmap

1. Open a [feature request](../../issues/new?template=feature_request.yml): what you want to do and
   why it helps.
2. Others give it a 👍; we answer in the issue.
3. Requests that help the most people, fit FigCraft and keep `.figcraft` files compatible with the
   official app go into *Ideas*, then into a release.
4. If FigCraft adds the feature itself, we use theirs and remove ours.

Bugs in FigCraft itself belong in [FigCraft's own tracker](https://github.com/storytold/figcraft/issues);
bugs in the add-ons go [here](../../issues/new?template=bug_report.yml).
