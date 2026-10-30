//! FigCraft Plus: units in number fields — rem (requested in storytold/figcraft#1325) and the
//! print units mm, cm, in and pt.
//!
//! - `mm`, `cm`, `in` and `pt` are what FigCraft's print export means by them: a px is a point
//!   (1/72 in), so a 210mm × 297mm frame exports as an A4 page.
//! - Every number field takes `rem`: `1.5rem` is 1.5 × the file's root size (`2rem + 4` works too).
//! - The text unit (px, rem or pt) and the root size belong to the file: Units (see `units_ui`,
//!   `prefs_plus_ui`). They are kept in the first page's layer data (`figcraftplus.remUnit`,
//!   `figcraftplus.remRoot`), so the official app opens the file as before; a file without them
//!   uses the defaults from Preferences ▸ FigCraft Plus.
//! - In rem (or pt), the font size, line height, letter spacing and paragraph spacing show in
//!   that unit; a bare number typed there is then in it too, and `px` still means px.
//!   Values are stored in px, as FigCraft stores them.
//! - Dev Mode's px → rem code uses the same root size.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use figcraft_doc::Document;

use crate::FigApp;
use crate::widgets::{self, FieldEdit};

/// The layer data key (on a page) holding the file's root size in px.
pub(crate) const PAGE_KEY: &str = "figcraftplus.remRoot";
/// The layer data key (on a page) holding the file's text unit ("px", "rem" or "pt").
pub(crate) const UNIT_KEY: &str = "figcraftplus.remUnit";

const DEFAULT_ROOT: f64 = 16.0;

static ROOT: AtomicU64 = AtomicU64::new(0x4030_0000_0000_0000); // 16.0

thread_local! {
    /// The last text a number field parsed named a length unit (`px`, `rem`).
    static EXPLICIT: Cell<bool> = const { Cell::new(false) };
}

/// The open file's root size in px.
pub(crate) fn root() -> f64 {
    let r = f64::from_bits(ROOT.load(Ordering::Relaxed));
    if r.is_finite() && r > 0.0 { r } else { DEFAULT_ROOT }
}

/// Whether text sizes show in a unit other than px.
pub(crate) fn show() -> bool {
    crate::units_ui::text() != "px"
}

/// px in one text unit.
fn k() -> f64 {
    crate::units_ui::text_k()
}

/// Whether the last text a number field parsed named its unit (and forget it).
pub(crate) fn take_explicit() -> bool {
    EXPLICIT.with(|x| x.replace(false))
}

#[cfg(test)]
pub(crate) fn set_root_for_test(r: f64) {
    ROOT.store(r.to_bits(), Ordering::Relaxed);
}

/// The root size a document sets (its first page that has one), else the preference.
pub(crate) fn doc_root(doc: &Document) -> f64 {
    doc.pages
        .iter()
        .find_map(|p| doc.node(*p)?.plugin_data.get(PAGE_KEY)?.trim().parse::<f64>().ok())
        .filter(|r| r.is_finite() && *r > 0.0 && *r <= 1000.0)
        .unwrap_or_else(|| Some(crate::prefs_plus_ui::get().root).filter(|r| r.is_finite() && *r > 0.0).unwrap_or(DEFAULT_ROOT))
}

/// Each frame: the open file's root size and units (and a unit picked from a field's chip).
pub(crate) fn tick(app: &mut FigApp, ctx: &egui::Context) {
    crate::units_ui::apply_pending(app, ctx);
    let doc = app.session.doc().ok();
    let r = doc.map(doc_root).unwrap_or(DEFAULT_ROOT);
    ROOT.store(r.to_bits(), Ordering::Relaxed);
    crate::units_ui::tick(doc);
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
        let r = k();
        return widgets::eval(text, cur.map(|c| c / r)).map(|v| v * r);
    }
    widgets::eval(text, cur)
}

/// A line height or letter spacing typed in rem (or a bare number on a px value, while sizes show
/// in rem): px. None leaves the text to FigCraft (auto, %, px). `px_now`: the value is in px.
pub(crate) fn spacing(text: &str, px_now: bool) -> Option<f64> {
    let t = text.trim();
    let lower = t.to_ascii_lowercase();
    if lower.contains("rem") || lower.ends_with("pt") || (show() && px_now && bare(t)) {
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
        Ok(px) if px.is_finite() => format!("{}{}", num(px / k()), crate::units_ui::text()),
        _ => text,
    }
}

fn num(v: f64) -> String {
    crate::units_ui::num(v)
}

// ---- the font size field --------------------------------------------------------------------------

/// The value the font size field shows: as it is. The field is a text length (`units_ui`): it
/// shows and takes the text unit like every other length field.
pub(crate) fn shown(px: Option<f64>) -> Option<f64> {
    px
}

/// The font size field's suffix: none (the field's unit chip shows the unit).
pub(crate) fn suffix() -> &'static str {
    ""
}

/// The font size field's edit: already px (`units_ui::back`).
pub(crate) fn to_px(e: Option<FieldEdit>) -> Option<FieldEdit> {
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rem_is_the_root_size() {
        // storytold/figcraft#1325
        ROOT.store(16.0f64.to_bits(), Ordering::Relaxed);
        crate::units_ui::set_for_test("px", "px");
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
        crate::units_ui::set_for_test("px", "rem");
        assert_eq!(label("24".into()), "2.4rem");
        assert_eq!(label("Auto".into()), "Auto");
        assert_eq!(spacing("2", true), Some(20.0), "a bare number is rem while sizes show in rem");
        assert_eq!(spacing("160", false), None, "a percentage stays FigCraft's");
        assert_eq!(length("+1", Some(20.0)), Some(30.0), "relative edits in rem");
        begin_parse();
        let typed = widgets::eval("18px", None);
        assert_eq!(typed, Some(18.0));
        assert!(take_explicit(), "a typed unit is noted, so the field keeps px as px");
        crate::units_ui::set_for_test("px", "px");
        ROOT.store(16.0f64.to_bits(), Ordering::Relaxed);
    }
}
