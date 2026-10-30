# FigCraft Plus — guide for AI agents

Read this first in every new session. It holds what a previous agent learned building this
project with its owner, so you can carry on without rediscovering it. Then read
[`ROADMAP.md`](ROADMAP.md) (what's next) and the top of [`CHANGELOG.md`](CHANGELOG.md) (what just
shipped).

## 1. What this is

**FigCraft Plus** adds professional colour, design-system and design-to-code tools to
[FigCraft](https://github.com/storytold/figcraft), an open-source Rust/egui design editor. This
repository holds **only the additions**. A GitHub Actions workflow checks out each official
FigCraft release, applies them, runs the tests and publishes a portable Windows build under
Releases as `v<figcraft>-plus.<N>`, named "FigCraft Plus N · FigCraft vX".

- Repository: `promocot/FigCraft-Plus` (formerly `Figcraft-addon` and `figcraft-oklch-support`;
  GitHub redirects the old names).
- Upstream: `storytold/figcraft`, and it moves fast. Its own `AGENTS.md`, `ROADMAP.md` and
  `docs/gaps.md` say what it plans next. Read them before planning, so Plus doesn't build what
  FigCraft is about to ship.
- Our three specialities: **Advanced colour · Design systems · Design to code.**

### Working with the owner

- The owner is a designer and frontend developer. **Talk to him in Russian.** Repository text
  (README, docs, code, commits) is in English.
- He often relays answers from another AI reviewer. Give an honest technical assessment: say
  where you agree, where you'd do otherwise and why. Don't just comply.
- When he says not to develop yet («не разрабатывай»), only plan, assess or edit docs.
- For big or ambiguous work, ask a short question first, with a recommended option. Then work
  through it with a task list.
- Release only after the tests pass **and** you have checked the feature in the running app
  (section 5). Report briefly: what shipped, what you verified, any deliberate deviation and why.

## 2. Non-negotiables

1. **Never name the commercial design tool FigCraft is modelled on** (the incumbent its own docs
   compare against) — not in the README, docs, changelog, commit messages, UI strings, release
   notes, topics or images, and not in this file. The owner wants zero legal exposure. Compare only
   with FigCraft ("In FigCraft v0.12.1 …"). Upstream FigCraft's own files may name it; ours never
   do. Before every push: `grep -rIni "<that name>" --exclude-dir=.git .` must find nothing.
2. **Files stay compatible with the official FigCraft, both ways.**
   - Never change FigCraft's document model or file format. `scripts/apply.py` stops the build if
     any overlay file or hook targets `crates/doc/` or `crates/format/`.
   - Plus data goes into fields FigCraft already keeps as text: layer/page `plugin_data`
     (`figcraftplus.*` keys) and variable/style descriptions.
   - Values are stored as FigCraft stores them (lengths in px). Units, formulas and settings are
     only presentation or derivation on top.
3. **Add only what FigCraft lacks.** When an official release ships a feature, remove ours: delete
   its hook and overlay parts, and move it to *Now in FigCraft itself* in the README. Before
   proposing anything, grep the upstream source — FigCraft already has a lot (see *Already in
   FigCraft* in the roadmap).
4. **No third-party assets.** No icons, images or UI kits from other products. Images in `docs/`
   are our own renders and screenshots of our own app.
5. **`main` is the release branch.** A push to `main` that touches `overlay/`, `hooks/`,
   `scripts/`, `CUSTOM_VERSION` or `build.yml` builds and publishes. Work in progress goes on a
   branch (`plus-N`, `support`, …). Push WIP there when a hook asks you to commit.
6. **Some account actions belong to the owner**, and the session proxy blocks most of them:
   - repository topics, social preview, renames;
   - deleting releases or tags;
   - GitHub Sponsors and Ko-fi.

   Explain the clicks to him instead. GitHub's GraphQL API is blocked here: use REST
   (`gh api repos/promocot/FigCraft-Plus/...`). `gh release list` fails, `gh api .../releases`
   works.

## 3. Repository map

```
overlay/crates/<crate>/src/…   our own source files, copied into FigCraft at the same paths
hooks/NNNN-name.json           where they connect to FigCraft's code (see §4)
scripts/apply.py               copies overlay/, applies hooks/, guards the file format
scripts/dev/fc.py              control-channel client: drive the running app, take screenshots
CUSTOM_VERSION                 the Plus number N (bump for every release)
CHANGELOG.md                   per-release notes; the release text is taken from it (§6)
ROADMAP.md                     staged plan by FigCraft version
docs/images/                   README screenshots, GIFs, hero; docs/social-preview.png
.github/workflows/build.yml    plan → test (Linux) → windows build → release
.github/ISSUE_TEMPLATE/        feature request, bug report, links
```

### The add-ons (overlay modules)

| Module | What it does |
|---|---|
| `color/oklch_profile.rs` | OKLCH ⇄ RGB in the file's profile (sRGB / Display P3), gamut mapping (CSS Color 4) |
| `engine/oklch_links.rs` | Colour channels bound to number variables; **derived colours** (`oklch(from {X} …)`, `color-mix`) and **calculated numbers** (`calc({A} * 2)`) kept in variable descriptions / layer data, resolved in every mode, cycle-safe |
| `engine/cmd/var_order.rs` | Reordering and sorting variables, groups, collections (also as commands) |
| `ui-egui/oklch_fields.rs` | Colour picker models (Hex, RGB, HSL, HSB, LCH, OKLCH, CMYK, CSS), the OKLCH plane and hue strip, captions, resizable width, the CSS field with formulas |
| `ui-egui/color_tools_ui.rs` | CMYK row (same naive formula as FigCraft's CMYK PDF export), *Copy as*, WCAG/APCA contrast against the layers really under the layer |
| `ui-egui/picker_place_ui.rs` | Where the picker opens; follows the selection to the same slot; closes on an outside click or Escape |
| `ui-egui/picker_libraries_ui.rs` | The picker's **Libraries** tab: colour variables filtered by the scope of what is edited |
| `ui-egui/var_picker_ui.rs` | One variable picker card everywhere (`card`, `embedded`, `offered`, `apply`); scopes |
| `ui-egui/units_ui.rs` | **Units**: per-document layout unit (px, rem, mm, cm, in, pt) and text unit (px, rem, pt); which fields are lengths; the field wrapper's conversion; the unit chip and right-click unit menu; canvas size label and measurements |
| `ui-egui/rem_ui.rs` | Root size, parsing typed units (`210mm`, `1.5rem`, the "explicit unit" flag), text-unit helpers |
| `ui-egui/prefs_plus_ui.rs` | Preferences ▸ FigCraft Plus (`figcraft-plus.json` next to the exe: units, root size, decimals, picker model); the Units popover |
| `ui-egui/popover_ui.rs` | The one popover look (frame, header, close button, divider) and closing rules; `PanelMenus` draws FigCraft's dark style menu in panel colours |
| `ui-egui/style_dialog_ui.rs`, `styles_plus_ui.rs`, `style_groups_ui.rs` | Create/edit styles in a dialog with preview; the **+** on Local styles; nested style groups |
| `ui-egui/text_vars_ui.rs` | Weight, paragraph spacing and indent from variables |
| `ui-egui/fonts_ui.rs` | Portable `fonts` folder, *Add fonts…*, *In this file* |
| `ui-egui/var_order_ui.rs` | Drag and drop and sorting in the Variables editor |
| `ui-egui/oklch_picker_tests.rs` | Headless egui tests of the picker |

Page `plugin_data` keys: `figcraftplus.layoutUnit`, `figcraftplus.remUnit` (the **text** unit:
px, rem or pt — the name is historical), `figcraftplus.remRoot`.

## 4. How add-ons attach (hooks)

A hook file is a JSON list of edits to FigCraft's source:

```json
{"note": "…", "file": "crates/ui-egui/src/lib.rs", "after": "^pub mod windows;$", "insert": "mod units_ui;"}
{"note": "…", "file": "crates/…/x.rs", "find": "exact text, once", "replace": "new text"}
```

- `python3 scripts/apply.py <figcraft-dir>` applies everything. It skips hooks whose result is
  already there, so it is safe to re-run. `--check` lists every hook that no longer fits instead of
  stopping at the first.
- Keep hooks **small**: one line to call into an overlay module, which holds the logic. This keeps
  porting to a new FigCraft version cheap.
- If you **edit** an existing hook's text, re-apply on a *clean* tree. Otherwise both the old and
  the new insert end up there:
  `git -C <figcraft> checkout -- . && git -C <figcraft> clean -fdq crates && python3 scripts/apply.py <figcraft>`.
- UI strings: every `tr("…")` key must exist in `crates/ui-egui/locales/en.toml` **and** be used.
  FigCraft's i18n test fails on unknown or unused keys. Add them through a locale hook, in English
  and Russian (see `0013-rem.json`).
- `0012-upstream-tests.json` marks FigCraft tests that conflict with intended Plus behaviour as
  `#[ignore = "FigCraft Plus: why"]`. Add to it only with a reason; prefer fixing our code.

**On a new FigCraft release:** run `apply.py --check` and fix the anchors that moved. Grep for
renamed APIs, build, run the tests, and check the README *Now in FigCraft itself* list against the
new release notes.

## 5. Local development loop

```sh
git clone --depth 1 --branch v0.12.1 https://github.com/storytold/figcraft work   # the tag CI builds
python3 scripts/apply.py work
cd work
cargo build --bin figcraft             # dev profile: ~11 min the first time, ~2 min after
cargo test -p figcraft-ui-egui --lib   # ~45 s once built; CI also runs the engine/color tests in build.yml
```

Build in the background and keep working. A full UI test run must pass before a release.

**Check every feature in the running app** (Linux, no display):

```sh
Xvfb :99 -screen 0 1440x900x24 &
DISPLAY=:99 HOME=/tmp/fc-home ./target/debug/figcraft --control 7979 &
```

Then drive the app with `scripts/dev/fc.py`: engine commands, clicks, typing, screenshots (docstring
inside). Lessons learned:

- Screenshot paths must be **absolute**.
- Look at the screenshot to find click targets. If the interface size isn't 100%, set
  `fc.SCALE` (points = pixels / scale).
- `figcraft-plus.json` lives next to the binary (`target/debug/`). Write it before launching to
  set preferences (`{"layoutUnit": "mm", "textUnit": "pt", "precision": 2, "colorModel": "OKLCH"}`).
- Stop the app with `pkill -x figcraft` (`pkill -f figcraft` kills your own shell).
- GIFs: snapshot frames with `c.shot`, then ffmpeg with per-frame palettes
  (`palettegen=stats_mode=single`, `paletteuse=new=1`). Crop to the subject; keep each GIF under
  ~2.5 MB.

### Pitfalls that cost time before

- **Don't put clickable widgets inside FigCraft's own fields in px.** Upstream tests click the
  middle of fields. Make extra affordances appear only in Plus modes, or use a right-click.
- Extra interactive areas next to fields use `Sense::CLICK`, not `Sense::click()`. The latter is a
  Tab stop and breaks Tab/Shift+Tab between fields.
- Two widgets with the same labels in one scope clash on ids (red error text in the UI). Wrap them
  in `ui.push_id(...)`.
- A popover opened by a click must ignore that same click as "outside" (remember the frame it
  toggled in).
- `ctx.read_response(id)` gives a widget's response before it is drawn (rects from the last pass).
  Useful for context menus on FigCraft's widgets.
- Number formatting for display goes through `widgets::fmt_num`, which honours *Decimals shown*.
  Never round stored values.

## 6. Releasing

1. Bump `CUSTOM_VERSION` to N.
2. `CHANGELOG.md`: add `## Plus N · FigCraft vX — YYYY-MM-DD` at the top. The workflow copies this
   section into the GitHub release notes, so write it for users. Mark items 🟣 *Exclusive* (FigCraft
   has nothing like it) or 🔵 *Enhanced* (FigCraft has a basic version).
3. `ROADMAP.md`: tick or move the items, and add a row to *Done*.
4. `README.md`:
   - the version line under the hero;
   - the feature's section (FigCraft vs FigCraft Plus quote, screenshot, bullets);
   - its row in *Feature comparison*;
   - the TOC.

   Screenshots go to `docs/images/NN-name.png`; the next free number is in the folder.
5. Push to the branch, merge to `main` (fast-forward), push. Watch the run:
   `gh api "repos/promocot/FigCraft-Plus/actions/runs?per_page=1"`, then the release via
   `gh api "repos/promocot/FigCraft-Plus/releases?per_page=1"`.
6. Docs-only pushes (README, ROADMAP, CHANGELOG, images) don't build. Changing `build.yml`
   rebuilds the current version and refreshes its notes.

## 7. Style of the docs

- Plain, concrete English. Say what the user sees and does, in UI words ("Preferences ▸ FigCraft
  Plus ▸ Decimals shown").
- Plus's own UI strings and the README prose say "colour" (British); FigCraft's strings say
  "Color"; the hero image says "color". Don't mass-rename: text must match what the app shows.
- README order: hero → badges → version line → Why (3 pillars) → See it in action (GIFs) →
  Compatibility → Roadmap block → Contents → Feature comparison → Download and update → Requested
  in FigCraft's issues → Now in FigCraft itself → Add-ons sections → Community → FAQ → How it works.

## 8. Where things stand (update this section when you finish work)

- **Released:** Plus 8 on FigCraft v0.12.1 (2026-10-30). Units everywhere: a unit chip and a
  right-click unit menu on length fields, one popover look and the same closing rules, Decimals
  shown 0–4.
- **Next — roadmap Stage 1 (on 0.12):**
  - rulers in the document's unit;
  - Dev Mode showing the document unit, with **CSS code in its own unit** (px or rem — never mm);
  - number variables shown with their unit by role (FigCraft's `engine/assets/token_roles.rs`
    already infers roles);
  - `em` for letter spacing and line height.
- **After 0.13:**
  - richer CSS variable export (FigCraft already exports JSON, CSS in px, Tailwind and Dart, and
    imports JSON; Plus adds `oklch()`, `rem`, relative colours, `calc()`, and CSS import);
  - bulk editing and binding of text styles;
  - Design System Inspector.
- **After 0.14:** %, vw, vh; auto layout from variables; line height % tokens; gradient tokens.
- **Known gaps:**
  - The style menu header has no close button (it's FigCraft's header with its own buttons).
  - Style dialogs keep their own closing rules (an outside click must not lose a style being
    created).
  - Some effect fields (noise, texture), grid tracks and prototype offsets stay in px.
- **Parked:**
  - Branch `support` holds a "Support FigCraft Plus" section and `.github/FUNDING.yml` with a
    `KOFI_NAME` placeholder. It waits for the owner's GitHub Sponsors approval or Ko-fi name.
  - A duplicated component outside a frame doesn't create an instance — an upstream bug; re-check
    after FigCraft 0.13 before touching it.
  - Repository topics and the social preview must be set by the owner in the GitHub UI.
