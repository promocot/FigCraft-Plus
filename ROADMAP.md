# FigCraft Plus — Roadmap

What we are working on, what comes next, and how to get a feature onto the list.

FigCraft Plus follows the official FigCraft releases. Each time a new FigCraft version comes out,
we first make sure every add-on still works on it, remove what FigCraft now does itself, and then
continue down this list. There are no dates: a step starts when the previous one is out.

**Want something?** [Ask for a feature](../../issues/new?template=feature_request.yml) — and give a
👍 to the requests you'd like too. The most wanted ones move up.

---

## 🔜 Next — FigCraft Plus 6: design system ↔ code

*Starts after FigCraft 0.13, once the add-ons are checked on it.*

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Export variables as CSS** — `oklch()` colours, `rem` sizes, derived colours as relative colours (`oklch(from var(--primary) l c h / 50%)`), calculated numbers as `calc()` | Hand developers exactly what the design uses, with no rounding to hex or px |
| ⬜ | **Import CSS custom properties as variables** — `--space-m: 1rem`, `oklch()`, `var()` (as an alias), `calc()` (as a calculated variable), `:root` and theme blocks as modes | Bring a site's design system into FigCraft in one step |
| ⬜ | **Remember missing-font replacements** — "always use Inter for Helvetica" | Open files from others without choosing the same replacements again |
| ⬜ | **Edit several text styles at once** — select styles in Local styles, change font, size or line height for all | Restyle a whole type scale in one go |

## 🧭 Then — FigCraft Plus 7: layout

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Sizes in % of the parent** — width 50%, height 100%, kept in step when the parent changes | Responsive layouts without recalculating by hand; files stay plain px for the official app |
| ⬜ | **Auto layout direction from a variable** — a column in Mobile, a row in Desktop | One responsive component instead of two |
| ⬜ | **Align to a key object** — select several layers, click one: the others align to it and it stays put | Precise layout without moving the reference |

## 📦 Later

| | Feature | What it gives you |
|---|---|---|
| ⬜ | **Export presets** — a set of sizes, formats and a file-name pattern (`{frame}_{w}x{h}`), exported in one click | Product cards and banners in every size at once |

## 💡 Ideas we are considering

Not planned yet — a 👍 on a [feature request](../../issues?q=is%3Aissue+label%3Aenhancement) helps
decide.

- OKLCH tonal scale generator: 50 … 950 from one colour, as variables that follow it
- Design tokens (DTCG) import and export
- A key to hide the selection outline while editing
- Remappable keyboard shortcuts

## ✅ Done

| Release | Highlights |
|---|---|
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
