//! FigCraft Plus: units in number fields — rem (requested in storytold/figcraft#1325) and the
//! print units mm, cm, in and pt.
//!
//! - `mm`, `cm`, `in` and `pt` are what FigCraft's print export means by them: a px is a point
//!   (1/72 in), so a 210mm × 297mm frame exports as an A4 page.
//! - Every number field takes `rem`: `1.5rem` is 1.5 × the file's root size (`2rem + 4` works too).
//! - The unit (px or rem) and the root size belong to the file: Typography ▸ gear (see
//!   `prefs_plus_ui`). They are kept in the first page's layer data (`figcraftplus.remUnit`,
//!   `figcraftplus.remRoot`), so the official app opens the file as before; a file without them
//!   uses the defaults from Preferences ▸ FigCraft Plus.
//! - In rem, the font size, line height, letter spacing and paragraph spacing show in rem; a bare
//!   number typed there is then rem too, and `px` still means px.
//!   Values are stored in px, as FigCraft stores them.
//! - Dev Mode's px → rem code uses the same root size.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use figcraft_doc::Document;

use crate::FigApp;
use crate::widgets::{self, FieldEdit};

/// The layer data key (on a page) holding the file's root size in px.
pub(crate) const PAGE_KEY: &str = "figcraftplus.remRoot";
/// The layer data key (on a page) holding the file's typography unit ("px" or "rem").
pub(crate) const UNIT_KEY: &str = "figcraftplus.remUnit";

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

/// The root size a document sets (its first page that has one), else the preference.
pub(crate) fn doc_root(doc: &Document) -> f64 {
    doc.pages
        .iter()
        .find_map(|p| doc.node(*p)?.plugin_data.get(PAGE_KEY)?.trim().parse::<f64>().ok())
        .filter(|r| r.is_finite() && *r > 0.0 && *r <= 1000.0)
        .unwrap_or_else(|| Some(crate::prefs_plus_ui::get().root).filter(|r| r.is_finite() && *r > 0.0).unwrap_or(DEFAULT_ROOT))
}

/// Whether a document's typography shows rem (its own choice, else the preference).
pub(crate) fn doc_rem(doc: &Document) -> bool {
    doc.pages
        .iter()
        .find_map(|p| doc.node(*p)?.plugin_data.get(UNIT_KEY).map(|u| u.trim().eq_ignore_ascii_case("rem")))
        .unwrap_or_else(|| crate::prefs_plus_ui::get().rem)
}

/// Each frame: the open file's root size and unit.
pub(crate) fn tick(app: &FigApp, _ctx: &egui::Context) {
    let (r, rem) = app.session.doc().map(|d| (doc_root(d), doc_rem(d))).unwrap_or((DEFAULT_ROOT, false));
    ROOT.store(r.to_bits(), Ordering::Relaxed);
    SHOW.store(rem, Ordering::Relaxed);
}

// ---- reading what was typed -------------------------------------------------------------------

/// A number field starts reading text.
pub(crate) fn begin_parse() {
    EXPLICIT.with(|e| e.set(false));
}

/// A number `v` typed with `unit`, in px: rem is the root size; mm, cm, in and pt are print
/// lengths (a px is a point, as in FigCraft's print export).
pub(crate) fn unit(v: f64, unit: &str) -> f64 {
    let k = match unit.to_ascii_lowercase().as_str() {
        "rem" => root(),
        "px" | "pt" => 1.0,
        "mm" => 72.0 / 25.4,
        "cm" => 72.0 / 2.54,
        "in" => 72.0,
        _ => return v,
    };
    EXPLICIT.with(|e| e.set(true));
    v * k
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
        // Print units, as FigCraft's PDF export reads them (A4 = 595.28 × 841.89 pt).
        let a4 = |s: &str| widgets::eval(s, None).map(|v| (v * 100.0).round() / 100.0);
        assert_eq!((a4("210mm"), a4("297 mm"), a4("21cm"), a4("1in"), a4("12pt")), (Some(595.28), Some(841.89), Some(595.28), Some(72.0), Some(12.0)));
        assert_eq!(widgets::eval("10mm + 2", None).map(|v| (v * 100.0).round() / 100.0), Some(30.35));
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
