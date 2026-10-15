//! FigCraft Plus: your own fonts and a font list without duplicates.
//!
//! - Font files (.ttf, .otf, .ttc) in a `fonts` folder next to the app are added at start, so a
//!   portable copy carries them; "Add fonts…" in the font list copies chosen files there.
//! - The font list has sections — Recent, In this file, Installed — and lists every family once,
//!   whatever its source (bundled with FigCraft, the system, the fonts folder); fonts from the
//!   fonts folder are tagged "Added", fonts a file uses that aren't here "Missing".

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};

use crate::FigApp;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets;

/// The font file types read.
const EXTS: [&str; 3] = ["ttf", "otf", "ttc"];

/// The folder of your fonts: `fonts` next to the app.
fn dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("fonts")))
}

fn is_font(p: &Path) -> bool {
    p.extension().and_then(|x| x.to_str()).is_some_and(|x| EXTS.iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// Every font file under `root` (subfolders too, a few levels deep).
fn font_files(root: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            if depth < 3 {
                font_files(&p, depth + 1, out);
            }
        } else if is_font(&p) {
            out.push(p);
        }
    }
}

/// Add the faces of the font file `path` that aren't known yet (a family and style already there,
/// bundled or installed, keeps its own). The families added.
fn install_file(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    let shared = std::sync::Arc::new(bytes);
    let count = face_count(&shared);
    let mut added = Vec::new();
    let mut db = figcraft_text::db().write().unwrap_or_else(PoisonError::into_inner);
    for i in 0..count {
        let Some(face) = figcraft_text::Face::from_bytes(figcraft_text::fontdb::FontBytes::Shared(shared.clone()), i, None, None) else { continue };
        let have = db.has_family(&face.family) && db.styles(&face.family).iter().any(|s| s.eq_ignore_ascii_case(&face.style));
        if have {
            continue;
        }
        if !added.iter().any(|f: &String| f.eq_ignore_ascii_case(&face.family)) {
            added.push(face.family.clone());
        }
        db.add(face);
    }
    drop(db);
    if !added.is_empty() {
        figcraft_text::clear_layout_cache();
    }
    added
}

/// How many fonts the file holds: a collection (`ttcf`) says so in its header, any other file is
/// one (or none, when it isn't a font: no face reads then).
fn face_count(bytes: &[u8]) -> u32 {
    match bytes.get(..12) {
        Some(h) if h.starts_with(b"ttcf") => h.get(8..12).and_then(|b| <[u8; 4]>::try_from(b).ok()).map_or(1, u32::from_be_bytes).min(64),
        _ => 1,
    }
}

// ---------------------------------------------------------------------------------------------
// State

#[derive(Default)]
struct State {
    started: bool,
    /// Lower-case names of the families added from the fonts folder.
    added: BTreeSet<String>,
    /// The families the open file uses.
    doc_families: Vec<String>,
    doc_key: Option<(u64, u64)>,
    /// "Add fonts…" was clicked: the next frame asks for files.
    want_files: bool,
    ctx: Option<egui::Context>,
}

fn state() -> MutexGuard<'static, State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(Default::default).lock().unwrap_or_else(PoisonError::into_inner)
}

fn note_added(families: Vec<String>) {
    let ctx = {
        let mut s = state();
        for f in families {
            s.added.insert(f.to_lowercase());
        }
        s.ctx.clone()
    };
    if let Some(c) = ctx {
        c.request_repaint();
    }
}

/// Each frame: at start add the fonts folder's fonts (in the background), note the open file's
/// families, and answer "Add fonts…".
pub(crate) fn tick(app: &mut FigApp, ctx: &egui::Context) {
    let start = {
        let mut s = state();
        let first = !s.started;
        s.started = true;
        s.ctx.get_or_insert_with(|| ctx.clone());
        first
    };
    if start && let Some(root) = dir() {
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let mut files = Vec::new();
            font_files(&root, 0, &mut files);
            let added: Vec<String> = files.iter().flat_map(|f| install_file(f)).collect();
            note_added(added);
        });
        #[cfg(target_arch = "wasm32")]
        drop(root);
    }
    if std::mem::take(&mut state().want_files) {
        add_fonts(app);
    }
    let Some(d) = app.session.docs.get(app.session.active) else { return };
    let key = (d.uid, d.revision);
    if state().doc_key == Some(key) {
        return;
    }
    let mut fams: Vec<String> = Vec::new();
    let mut note = |f: &str| {
        if !f.is_empty() && !fams.iter().any(|x| x.eq_ignore_ascii_case(f)) {
            fams.push(f.to_string());
        }
    };
    for n in d.doc.nodes() {
        if let Some(t) = n.kind.text() {
            note(&t.style.font_family);
            for r in &t.runs {
                note(&r.style.font_family);
            }
        }
    }
    for s in &d.doc.styles {
        if let figcraft_doc::StyleKind::Text { style } = &s.kind {
            note(&style.font_family);
        }
    }
    fams.sort_by_key(|f| f.to_lowercase());
    let mut s = state();
    s.doc_key = Some(key);
    s.doc_families = fams;
}

/// Ask for font files, copy them into the fonts folder and add them.
fn add_fonts(app: &mut FigApp) {
    let Some(root) = dir() else { return };
    let Some(src) = app.services.pick_open(tr("Add fonts"), &EXTS) else { return };
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    let Some(name) = src.file_name() else { return };
    let dst = root.join(name);
    if dst != src && std::fs::copy(&src, &dst).is_err() {
        return;
    }
    note_added(install_file(&dst));
}

// ---------------------------------------------------------------------------------------------
// The font list

const ROW: f32 = 26.0;
const RECENT_KEY: &str = "figcraft_recent_fonts";

#[derive(Debug, PartialEq)]
enum Row {
    Header(String),
    /// A family on this computer; true when it came from the fonts folder.
    Installed(String, bool),
    /// A family the file uses that isn't here.
    Missing(String),
}

/// The rows for query `q` (lower case): Recent and In this file (without a query), then every
/// installed family once.
fn rows(q: &str, installed: &[String], recent: &[String], in_file: &[String], added: &BTreeSet<String>) -> Vec<Row> {
    let hit = |f: &str| q.is_empty() || f.to_lowercase().contains(q);
    let local: BTreeSet<String> = installed.iter().map(|f| f.to_lowercase()).collect();
    let mine = |f: &str| added.contains(&f.to_lowercase());
    let mut out = Vec::new();
    if q.is_empty() {
        let recent: Vec<&String> = recent.iter().filter(|f| local.contains(&f.to_lowercase())).collect();
        if !recent.is_empty() {
            out.push(Row::Header(tr("Recent").into()));
            out.extend(recent.into_iter().map(|f| Row::Installed(f.clone(), mine(f))));
        }
        if !in_file.is_empty() {
            out.push(Row::Header(tr("In this file").into()));
            for f in in_file {
                out.push(if local.contains(&f.to_lowercase()) { Row::Installed(f.clone(), mine(f)) } else { Row::Missing(f.clone()) });
            }
        }
    }
    let list: Vec<&String> = installed.iter().filter(|f| hit(f)).collect();
    if !list.is_empty() {
        out.push(Row::Header(tr("Installed").into()));
        out.extend(list.into_iter().map(|f| Row::Installed(f.clone(), mine(f))));
    }
    out
}

/// The searchable, sectioned family list (replaces FigCraft's flat one), with "Add fonts…" at the
/// bottom. Returns the family picked.
pub(crate) fn font_picker(ui: &mut Ui, current: Option<&str>, just_opened: bool) -> Option<String> {
    let t = Tokens::get(ui.ctx());
    let q_id = Id::new("font_query");
    let mut q: String = ui.data(|d| d.get_temp(q_id)).unwrap_or_default();
    if just_opened {
        q.clear();
    }
    let search = ui.add(egui::TextEdit::singleline(&mut q).hint_text(tr("Search fonts")).desired_width(ui.available_width()).font(regular(11.5)));
    if just_opened {
        search.request_focus();
    }
    ui.data_mut(|d| d.insert_temp(q_id, q.clone()));
    let ql = q.trim().to_lowercase();
    let installed = figcraft_text::family_names();
    let recent: Vec<String> = ui.ctx().data_mut(|d| d.get_persisted::<Vec<String>>(Id::new(RECENT_KEY))).unwrap_or_default();
    let list = {
        let s = state();
        rows(&ql, &installed, &recent, &s.doc_families, &s.added)
    };
    if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let first = list.iter().find_map(|r| match r {
            Row::Installed(f, _) | Row::Missing(f) => Some(f.clone()),
            Row::Header(_) => None,
        });
        if first.is_some() {
            return first;
        }
    }
    let mut picked = None;
    if list.iter().all(|r| matches!(r, Row::Header(_))) {
        widgets::label(ui, tr("No fonts match"), t.text_tertiary);
    } else {
        let mut budget = 4;
        egui::ScrollArea::vertical().max_height(360.0).auto_shrink([false, true]).show_rows(ui, ROW, list.len(), |ui, range| {
            for r in list.get(range).unwrap_or(&[]) {
                let w = ui.available_width();
                let (rect, resp) = ui.allocate_exact_size(vec2(w, ROW), if matches!(r, Row::Header(_)) { Sense::hover() } else { Sense::click() });
                match r {
                    Row::Header(h) => {
                        ui.painter().text(pos2(rect.min.x + 8.0, rect.center().y), Align2::LEFT_CENTER, h, semibold(10.5), t.text_secondary);
                    }
                    Row::Installed(f, _) | Row::Missing(f) => {
                        let hovered = resp.hovered();
                        if hovered {
                            ui.painter().rect_filled(rect, CornerRadius::same(5), t.accent);
                        }
                        let col = if hovered { Color32::WHITE } else { t.text };
                        let dim = if hovered { Color32::from_white_alpha(200) } else { t.text_tertiary };
                        if current.is_some_and(|c| c.eq_ignore_ascii_case(f)) {
                            icons::paint(
                                ui.painter(),
                                Rect::from_center_size(pos2(rect.min.x + 10.0, rect.center().y), vec2(12.0, 12.0)),
                                Icon::Check,
                                col,
                            );
                        }
                        let tag = match r {
                            Row::Installed(_, true) => Some(tr("Added")),
                            Row::Missing(_) => Some(tr("Missing")),
                            _ => None,
                        };
                        let tag_w = tag.map_or(0.0, |s| ui.painter().layout_no_wrap(s.to_string(), regular(10.0), dim).size().x + 10.0);
                        if let Some(s) = tag {
                            ui.painter().text(pos2(rect.max.x - 6.0, rect.center().y), Align2::RIGHT_CENTER, s, regular(10.0), dim);
                        }
                        let text_min = pos2(rect.min.x + 22.0, rect.min.y);
                        let max_w = rect.max.x - text_min.x - 6.0 - tag_w;
                        let tex = if matches!(r, Row::Missing(_)) { None } else { crate::font_preview::family(ui.ctx(), f, 13.0, &mut budget) };
                        match tex {
                            Some(tex) => {
                                crate::font_preview::paint(ui.painter(), &tex, pos2(text_min.x, rect.center().y), max_w, ROW, col);
                            }
                            None => {
                                ui.painter().with_clip_rect(Rect::from_min_size(text_min, vec2(max_w.max(0.0), ROW))).text(
                                    pos2(text_min.x, rect.center().y),
                                    Align2::LEFT_CENTER,
                                    f,
                                    regular(12.0),
                                    col,
                                );
                            }
                        }
                        if resp.clicked() {
                            picked = Some(f.clone());
                        }
                    }
                }
            }
        });
    }
    // "Add fonts…": your font files, kept in the fonts folder next to the app.
    ui.add_space(4.0);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, ROW), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(r.min.x + 10.0, r.center().y), vec2(12.0, 12.0)), Icon::Plus, t.icon);
    ui.painter().text(pos2(r.min.x + 22.0, r.center().y), Align2::LEFT_CENTER, tr("Add fonts…"), regular(11.5), t.text);
    if resp.on_hover_text(tr("Font files are kept in the fonts folder next to the app")).clicked() {
        state().want_files = true;
        ui.ctx().request_repaint();
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_each_family_once() {
        let installed = vec!["Arial".to_string(), "Inter".to_string(), "My Font".to_string()];
        let added: BTreeSet<String> = ["my font".to_string()].into();
        let label = |r: &Row| match r {
            Row::Header(h) => format!("# {h}"),
            Row::Installed(f, g) => format!("{f}{}", if *g { " +" } else { "" }),
            Row::Missing(f) => format!("{f} ?"),
        };
        let all: Vec<String> = rows("", &installed, &[], &[], &added).iter().map(label).collect();
        assert_eq!(all, ["# Installed", "Arial", "Inter", "My Font +"]);
        let start: Vec<String> =
            rows("", &installed, &["Inter".into(), "Gone".into()], &["Lora".into(), "Arial".into()], &added).iter().map(label).collect();
        assert_eq!(&start[..6], ["# Recent", "Inter", "# In this file", "Lora ?", "Arial", "# Installed"]);
        let found: Vec<String> = rows("in", &installed, &["Inter".into()], &[], &added).iter().map(label).collect();
        assert_eq!(found, ["# Installed", "Inter"]);
        assert_eq!(face_count(b"ttcf\0\x01\0\0\0\0\0\x03rest"), 3);
        assert_eq!(face_count(b"\0\x01\0\0"), 1);
        assert!(is_font(Path::new("a/B.TTF")) && is_font(Path::new("x.otf")) && !is_font(Path::new("x.woff2")));
    }

    #[test]
    fn reads_the_files_in_a_folder() {
        let root = std::env::temp_dir().join(format!("figcraftplus-fonts-{}", std::process::id()));
        let _ = std::fs::create_dir_all(root.join("sub"));
        std::fs::write(root.join("a.ttf"), b"x").unwrap();
        std::fs::write(root.join("sub").join("b.otf"), b"x").unwrap();
        std::fs::write(root.join("notes.txt"), b"x").unwrap();
        let mut files = Vec::new();
        font_files(&root, 0, &mut files);
        let names: Vec<String> = files.iter().filter_map(|p| p.file_name()?.to_str().map(String::from)).collect();
        assert_eq!(names, ["a.ttf", "b.otf"]);
        assert!(install_file(&root.join("a.ttf")).is_empty(), "not a font: nothing added");
        let _ = std::fs::remove_dir_all(&root);
    }
}
