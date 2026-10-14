# FigCraft Plus (OKLCH)

**[FigCraft](https://github.com/storytold/figcraft) with extra colour tools — OKLCH in the colour
picker, a resizable picker with labelled fields, and colours driven by number variables.**

This repository holds only the additions. A GitHub Actions workflow takes every new FigCraft
release, applies them, runs the tests and publishes a ready-to-run Windows build under
**[Releases](../../releases)** — usually within a day of the FigCraft release.

## Download

1. Open **[Releases](../../releases)** and take the latest
   `figcraft-<version>-custom.<n>-windows-x64-portable.zip`.
2. Unzip it anywhere (a USB drive works) and run `figcraft.exe`.
3. The build isn't code-signed: if Windows SmartScreen warns, choose **More info → Run anyway**.

To hear about new builds, use **Watch → Custom → Releases** on this page.

The build is FigCraft from the same release tag, built the same way (release profile, static C
runtime, embedded fonts, `figcraft-cli.exe` included), plus the features below. `.figcraft` files
stay fully compatible with the official app in both directions.

## Features

### OKLCH in the colour picker

The picker's model menu gains **OKLCH** (Hex / RGB / HSL / HSB / **OKLCH** / CSS):

| Field | Meaning | Range |
|---|---|---|
| **L** | lightness | 0–1 |
| **C** | chroma | 0–0.4 |
| **H** | hue | degrees |

- Same units and precision as [oklch.com](https://oklch.com) and CSS `oklch(L C H)`: paste a hex
  copied from FigCraft into oklch.com and you get the same numbers. Fields also take CSS forms
  (`64.56%` for L, `55%` for C = 0.22).
- Values are read and written in the file's colour profile (sRGB or Display P3,
  File ▸ Color profile). Colours outside the gamut are mapped in with the CSS Color 4 algorithm
  (chroma reduced, lightness and hue kept).
- In OKLCH mode the whole picker works in OKLCH: the HSB square becomes an **OKLCH plane**
  (chroma across, lightness down, at the current hue) with the exact gamut edge — and, in a
  Display P3 file, the sRGB edge dashed — and the rainbow becomes an **OKLCH hue strip** at the
  current lightness and chroma, so dragging the hue keeps the lightness.
- Verified against the [coloraide](https://github.com/facelessuser/coloraide) reference
  implementation (CSS Color 4) on 6,000 colours: largest difference 0.000001.

### A better picker

- **Captions over every field** in every model (R G B, H S L, H S B, L C H, A).
- **Width adapts** to the chosen model; drag the picker's **left edge** to make it wider
  (double-click the edge to reset).

### Colours driven by number variables

A colour variable's L, C and H can come from number variables — e.g. bind H to `Hue = 280`,
change `Hue` to 130 and every linked colour, and every layer using it, follows in one undo step.

Open the colour variable in the Variables editor, click its swatch, choose **OKLCH** and pick
variables under **Linked to number variables**.

- L from a variable is 0–1 (or 0–100 when above 1), C is 0–0.4, H is degrees.
- A number variable in the colour's own collection is read in the same mode, so Light and Dark
  (or Desktop and Mobile) can differ.
- The link is stored as a line in the variable's description, e.g. `oklch(0.62 0.19 {Hue})`; it
  can be edited by hand. Rename a number variable → update the name in the formula too.
- Editing the colour itself (hex, eyedropper) updates the formula's fixed channels and keeps the
  links. The official FigCraft simply keeps the last computed colours.

## How it works

```
overlay/          the additions' own files, copied into FigCraft at the same paths
hooks/            where they connect: "insert after this line" / "replace this text"
scripts/apply.py  copies overlay/ and applies hooks/ to a FigCraft checkout
CUSTOM_VERSION    version of the additions (bump after changing them)
.github/workflows/build.yml   the daily build
```

Every day at 04:17 UTC the workflow checks FigCraft's latest release. If this repository hasn't
built it yet (or the additions changed), it checks the release out, runs `scripts/apply.py`, runs
the additions' tests and publishes `v<figcraft version>-custom.<CUSTOM_VERSION>` under Releases.

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
python3 scripts/apply.py figcraft
cd figcraft && cargo run --release -p figcraft
```

### Add a feature

1. Put its code in new files under `overlay/` (same paths as in FigCraft).
2. Connect them with a new `hooks/0002-….json` (see `0001` for the format).
3. Bump `CUSTOM_VERSION`. Pushing starts a build.

## License

FigCraft is MIT OR Apache-2.0; these additions are offered under the same terms. Not affiliated
with the ArtCraft team. FigCraft is a trademark of its owners; Figma is a trademark of Figma, Inc.
