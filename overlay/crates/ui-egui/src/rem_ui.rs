//! FigCraft Plus: rem units (requested in storytold/figcraft#1325).
//!
//! - Every number field takes `rem`: `1.5rem` is 1.5 × the file's root size (`2rem + 4` works too).
//! - The root size (16 px unless set) belongs to the file: Typography ▸ Type settings ▸ Root size.
//!   It is kept in the first page's layer data (`figcraftplus.remRoot`), so the official app opens
//!   the file as before.
//! - "Show sizes in rem" (a preference) shows the font size, line height, letter spacing and
//!   paragraph spacing in rem; a bare number typed there is then rem too, and `px` still means px.
//!   Values are stored in px, as FigCraft stores them.
//! - Dev Mode's px → rem code uses the same root size.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use egui::{Id, Ui};
use figcraft_doc::Document;
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::widgets::{self, FieldEdit, Lead};

/// The layer data key (on a page) holding the file's root size in px.
pub(crate) const PAGE_KEY: &str = "figcraftplus.remRoot";

const DEFAULT_ROOT: f64 = 16.0;

static ROOT: AtomicU64 = AtomicU64::new(0x4030_0000_0000_0000); // 16.0
static SHOW: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The last text a number field parsed named a length unit (`px`, `rem`).
    static EXPLICIT: Cell<bool> = const { Cell::new(false) };
}

/// The open file's root size in px.
pub(crate) fn root() -> f64 {
    let r = f64::from_bits(ROOT.load(Ordering::Relaxed));
    if r.is_finite() && r > 0.0 { r } else { DEFAULT_ROOT }
}

/// Whether sizes show in rem.
pub(crate) fn show() -> bool {
    SHOW.load(Ordering::Relaxed)
}

/// The root size a document sets (its first page that has one), else 16.
pub(crate) fn doc_root(doc: &Document) -> f64 {
    doc.pages
        .iter()
        .find_map(|p| doc.node(*p)?.plugin_data.get(PAGE_KEY)?.trim().parse::<f64>().ok())
        .filter(|r| r.is_finite() && *r > 0.0 && *r <= 1000.0)
        .unwrap_or(DEFAULT_ROOT)
}

fn show_id() -> Id {
    Id::unique("figcraftplus_rem_show")
}

/// Each frame: the open file's root size and the display preference.
pub(crate) fn tick(app: &FigApp, ctx: &egui::Context) {
    let r = app.session.doc().map(doc_root).unwrap_or(DEFAULT_ROOT);
    ROOT.store(r.to_bits(), Ordering::Relaxed);
    SHOW.store(ctx.data_mut(|d| d.get_persisted::<bool>(show_id())).unwrap_or(false), Ordering::Relaxed);
}

// ---- reading what was typed -------------------------------------------------------------------

/// A number field starts reading text.
pub(crate) fn begin_parse() {
    EXPLICIT.with(|e| e.set(false));
}

/// A number `v` typed with `unit`: rem becomes px.
pub(crate) fn unit(v: f64, unit: &str) -> f64 {
    if unit.eq_ignore_ascii_case("rem") {
        EXPLICIT.with(|e| e.set(true));
        v * root()
    } else {
        if unit.eq_ignore_ascii_case("px") {
            EXPLICIT.with(|e| e.set(true));
        }
        v
    }
}

/// Whether `text` is a bare number or arithmetic (no unit, no %).
fn bare(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty() && !t.chars().any(|c| c.is_alphabetic() || c == '%')
}

/// A length typed into a text field that shows px or rem: px. `cur` is the value now (px).
pub(crate) fn length(text: &str, cur: Option<f64>) -> Option<f64> {
    if show() && bare(text) {
        let r = root();
        return widgets::eval(text, cur.map(|c| c / r)).map(|v| v * r);
    }
    widgets::eval(text, cur)
}

/// A line height or letter spacing typed in rem (or a bare number on a px value, while sizes show
/// in rem): px. None leaves the text to FigCraft (auto, %, px). `px_now`: the value is in px.
pub(crate) fn spacing(text: &str, px_now: bool) -> Option<f64> {
    let t = text.trim();
    if t.to_ascii_lowercase().contains("rem") || (show() && px_now && bare(t)) {
        return length(t, None).filter(|v| v.is_finite());
    }
    None
}

/// A field's label (a px value as text) in rem while sizes show in rem.
pub(crate) fn label(text: String) -> String {
    if !show() {
        return text;
    }
    match text.trim().parse::<f64>() {
        Ok(px) if px.is_finite() => format!("{}rem", num(px / root())),
        _ => text,
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

// ---- the font size field --------------------------------------------------------------------------

/// The value the font size field shows (rem while sizes show in rem).
pub(crate) fn shown(px: Option<f64>) -> Option<f64> {
    if show() { px.map(|v| v / root()) } else { px }
}

/// The font size field's unit.
pub(crate) fn suffix() -> &'static str {
    if show() { "rem" } else { "" }
}

/// The font size field's edit in px: a value shown in rem is scaled back, unless the text named
/// its unit (it is px already).
pub(crate) fn to_px(e: Option<FieldEdit>) -> Option<FieldEdit> {
    let explicit = EXPLICIT.with(|x| x.replace(false));
    if !show() {
        return e;
    }
    let r = root();
    match e? {
        FieldEdit::Set(v) if explicit => Some(FieldEdit::Set(v)),
        FieldEdit::Set(v) => Some(FieldEdit::Set(v * r)),
        FieldEdit::Scrub(v, done) => Some(FieldEdit::Scrub(v * r, done)),
    }
}

// ---- settings ---------------------------------------------------------------------------------------

/// Root size and "Show sizes in rem", at the top of Type settings.
pub(crate) fn settings(app: &mut FigApp, ui: &mut Ui) {
    let Ok(doc) = app.session.doc() else { return };
    let current = doc_root(doc);
    let page = doc.pages.first().copied();
    let field = crate::typography::field_row(ui, tr("Root size (rem)"), |ui, fw| {
        widgets::number_field(ui, Id::unique("figcraftplus_rem_root"), Lead::None, Some(current), "px", fw, 1.0)
    });
    if let (Some(FieldEdit::Set(v) | FieldEdit::Scrub(v, true)), Some(page)) = (field, page) {
        let v = v.clamp(1.0, 1000.0);
        let value = if (v - DEFAULT_ROOT).abs() < 1e-9 { serde_json::Value::Null } else { json!(widgets::fmt_num(v)) };
        app.run("node.set", json!({"ids": [page], "props": {"pluginData": {PAGE_KEY: value}}}));
    }
    let on = show();
    if widgets::checkbox(ui, on, tr("Show sizes in rem")) {
        ui.ctx().data_mut(|d| d.insert_persisted(show_id(), !on));
        SHOW.store(!on, Ordering::Relaxed);
    }
    ui.add_space(4.0);
    ui.separator();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rem_is_the_root_size() {
        // storytold/figcraft#1325
        ROOT.store(16.0f64.to_bits(), Ordering::Relaxed);
        SHOW.store(false, Ordering::Relaxed);
        assert_eq!(widgets::eval("1.5rem", None), Some(24.0));
        assert_eq!(widgets::eval("2rem + 4", None), Some(36.0));
        assert_eq!(widgets::eval("1.5 rem", None), Some(24.0));
        assert_eq!(spacing("1.5rem", true), Some(24.0));
        assert_eq!(spacing("150%", true), None, "FigCraft's own");
        assert_eq!(label("24".into()), "24");
        ROOT.store(10.0f64.to_bits(), Ordering::Relaxed);
        SHOW.store(true, Ordering::Relaxed);
        assert_eq!(label("24".into()), "2.4rem");
        assert_eq!(label("Auto".into()), "Auto");
        assert_eq!(spacing("2", true), Some(20.0), "a bare number is rem while sizes show in rem");
        assert_eq!(spacing("160", false), None, "a percentage stays FigCraft's");
        assert_eq!(length("+1", Some(20.0)), Some(30.0), "relative edits in rem");
        begin_parse();
        let typed = widgets::eval("18px", None);
        assert!(matches!(to_px(typed.map(FieldEdit::Set)), Some(FieldEdit::Set(v)) if v == 18.0), "px stays px");
        begin_parse();
        assert!(matches!(to_px(Some(FieldEdit::Set(1.5))), Some(FieldEdit::Set(v)) if v == 15.0));
        SHOW.store(false, Ordering::Relaxed);
        ROOT.store(16.0f64.to_bits(), Ordering::Relaxed);
    }
}
