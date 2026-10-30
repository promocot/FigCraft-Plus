//! FigCraft Plus: its preferences and the Typography settings popover (storytold/figcraft#1325's
//! options A and C).
//!
//! - **Preferences ▸ FigCraft Plus**: the typography unit (px or rem) new and unset documents use,
//!   the default root size, and the model the colour picker opens in (OKLCH, CMYK, …). Kept in
//!   `figcraft-plus.json` next to the app, so a portable copy carries them.
//! - **Typography ▸ gear**: this document's unit and root size (kept in the file).

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use egui::{Align2, Id, Ui};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, FieldEdit, Lead};

/// FigCraft Plus's own preferences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Prefs {
    /// Typography shows sizes in rem (else px) where a document doesn't say.
    pub rem: bool,
    /// The root size (px) where a document doesn't set one.
    pub root: f64,
    /// The colour picker's model when it opens ("Hex", "OKLCH", "CMYK", …).
    pub color_model: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { rem: false, root: 16.0, color_model: "Hex".into() }
    }
}

fn path() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|d| d.join("figcraft-plus.json"))
}

fn store() -> MutexGuard<'static, Prefs> {
    static P: OnceLock<Mutex<Prefs>> = OnceLock::new();
    P.get_or_init(|| {
        let p = path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Mutex::new(p)
    })
    .lock()
    .unwrap_or_else(PoisonError::into_inner)
}

/// The preferences now.
pub(crate) fn get() -> Prefs {
    store().clone()
}

fn set(f: impl FnOnce(&mut Prefs)) {
    let mut p = store();
    f(&mut p);
    if let (Some(path), Ok(text)) = (path(), serde_json::to_string_pretty(&*p)) {
        let _ = std::fs::write(path, text);
    }
}

/// The index in `models` of the model the picker opens in.
pub(crate) fn default_model(models: &[&str]) -> usize {
    let want = get().color_model;
    models.iter().position(|m| m.eq_ignore_ascii_case(&want)).unwrap_or(0)
}

// ---- Preferences ▸ FigCraft Plus --------------------------------------------------------------------

/// Two-way switch (px | rem); true when the second is picked now.
fn unit_switch(ui: &mut Ui, rem: bool) -> Option<bool> {
    let mut out = None;
    ui.horizontal(|ui| {
        for (on, label) in [(false, "px"), (true, "rem")] {
            if ui.selectable_label(rem == on, label).clicked() && rem != on {
                out = Some(on);
            }
        }
    });
    out
}

const MODELS: [&str; 8] = ["Hex", "RGB", "HSL", "HSB", "LCH", "OKLCH", "CMYK", "CSS"];

/// The FigCraft Plus section of Preferences.
pub(crate) fn preferences(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let p = get();
    ui.add_space(8.0);
    ui.separator();
    ui.label(egui::RichText::new("FigCraft Plus").font(semibold(11.5)).color(t.text));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(tr("Typography unit"));
        if let Some(rem) = unit_switch(ui, p.rem) {
            set(|p| p.rem = rem);
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Default root size"));
        if let Some(FieldEdit::Set(v) | FieldEdit::Scrub(v, true)) =
            widgets::number_field(ui, Id::unique("figcraftplus_pref_root"), Lead::None, Some(p.root), "px", 80.0, 1.0)
        {
            set(|p| p.root = v.clamp(1.0, 1000.0));
        }
    });
    widgets::label(ui, tr("Applies to new documents. Each file stores its own root size."), t.text_secondary);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(tr("Colour picker opens in"));
        let resp = widgets::dropdown(ui, &p.color_model, 100.0, None);
        crate::right::menu_popup(&resp).show(|ui| {
            for m in MODELS {
                if crate::right::menu_check(ui, p.color_model.eq_ignore_ascii_case(m), m, "") {
                    set(|p| p.color_model = m.to_string());
                    ui.close();
                }
            }
        });
    });
}

// ---- Typography ▸ settings ----------------------------------------------------------------------------

fn popover_id() -> Id {
    Id::unique("figcraftplus_typography_settings")
}

/// The Typography header's settings button was clicked (`clicked`): open or close the popover;
/// draw it while open.
pub(crate) fn typography_settings(app: &mut FigApp, ui: &mut Ui, clicked: bool) {
    let ctx = ui.ctx().clone();
    let mut at: Option<egui::Pos2> = ctx.data(|d| d.get_temp(popover_id()));
    if clicked {
        // A frame may lay the panel out twice: toggle once per frame.
        let frame = ctx.cumulative_frame_nr();
        if ctx.data(|d| d.get_temp::<u64>(popover_id().with("toggled"))) == Some(frame) {
            return;
        }
        ctx.data_mut(|d| d.insert_temp(popover_id().with("toggled"), frame));
        at = match at {
            Some(_) => None,
            // Left of the properties panel, level with the Typography header.
            None => Some(egui::pos2(ui.max_rect().left() - 292.0, (ui.cursor().top() - 40.0).max(48.0))),
        };
        ctx.data_mut(|d| match at {
            Some(p) => {
                d.insert_temp(popover_id(), p);
            }
            None => {
                d.remove_temp::<egui::Pos2>(popover_id());
            }
        });
        return;
    }
    let Some(at) = at else { return };
    let Ok(doc) = app.session.doc() else { return };
    let page = doc.pages.first().copied();
    let root = crate::rem_ui::doc_root(doc);
    let rem = crate::rem_ui::doc_rem(doc);
    let t = Tokens::get(&ctx);
    let mut close = false;
    // Inside the window: it is about 240 points tall.
    let screen = ctx.content_rect();
    let at = egui::pos2(at.x.max(screen.left() + 8.0), at.y.min(screen.bottom() - 250.0).max(screen.top() + 8.0));
    let area = egui::Area::new(popover_id().with("area")).order(egui::Order::Foreground).fixed_pos(at).show(&ctx, |ui| {
        egui::Frame::popup(&ctx.global_style()).fill(t.panel).corner_radius(egui::CornerRadius::same(10)).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.set_width(268.0);
            ui.label(egui::RichText::new(tr("Typography settings")).font(semibold(12.0)).color(t.text));
            ui.label(egui::RichText::new(tr("Applies to this document")).font(regular(10.5)).color(t.text_secondary));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(tr("Unit")).font(regular(11.0)).color(t.text));
            if let (Some(on), Some(page)) = (unit_switch(ui, rem), page) {
                app.run("node.set", json!({"ids": [page], "props": {"pluginData": {crate::rem_ui::UNIT_KEY: if on { "rem" } else { "px" }}}}));
            }
            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr("Root size (1rem =)")).font(regular(11.0)).color(t.text));
            if let (Some(FieldEdit::Set(v) | FieldEdit::Scrub(v, true)), Some(page)) =
                (widgets::number_field(ui, Id::unique("figcraftplus_rem_root"), Lead::None, Some(root), "px", 240.0, 1.0), page)
            {
                let v = v.clamp(1.0, 1000.0);
                app.run("node.set", json!({"ids": [page], "props": {"pluginData": {crate::rem_ui::PAGE_KEY: widgets::fmt_num(v)}}}));
            }
            ui.add_space(6.0);
            let note = format!("1rem = {}px, {}", widgets::fmt_num(root), tr("stored in the file"));
            ui.label(egui::RichText::new(note).font(regular(10.5)).color(t.text_secondary));
            ui.label(egui::RichText::new(tr("Default for new documents: Preferences")).font(regular(10.5)).color(t.text_secondary));
        });
    });
    // A click outside closes it; Esc too.
    let outside = ctx.input(|i| i.pointer.primary_clicked())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !area.response.rect.contains(p));
    if outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if close {
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(popover_id()));
    }
    let _ = Align2::LEFT_TOP;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_read_and_pick_the_picker_model() {
        let p: Prefs = serde_json::from_str(r#"{"rem": true, "colorModel": "OKLCH"}"#).unwrap();
        assert_eq!((p.rem, p.root, p.color_model.as_str()), (true, 16.0, "OKLCH"), "missing keys take defaults");
        assert_eq!(MODELS.iter().position(|m| m.eq_ignore_ascii_case(&p.color_model)), Some(5));
    }
}
