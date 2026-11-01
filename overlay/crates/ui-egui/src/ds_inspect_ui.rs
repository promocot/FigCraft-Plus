//! FigCraft Plus: the Design System Inspector (`designSystem.inspect`) — **Inspect** in the
//! Variables editor's header.
//!
//! A popover lists what the inspector found, grouped (Not on a variable, Broken references,
//! Circular formulas, Unused variables, Duplicate tokens, Without a scope), each group with its
//! count and, where the findings can be fixed, **Fix all** (one undo step). A row shows the
//! finding; clicking it jumps to its layer (selected and zoomed to) or its variable (the
//! Variables editor on its collection, its details open); **Fix** fixes that one. The list
//! refreshes after every change.

use egui::{Id, Response, RichText, Sense, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, tr_noop, trf};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets;

fn open_id() -> Id {
    Id::unique("figcraftplus_ds_inspector")
}

const W: f32 = 380.0;

/// The groups, in order: (finding kinds, title).
const GROUPS: [(&[&str], &str); 6] = [
    (&["brokenAlias", "brokenFormula", "invalid"], tr_noop("Broken references")),
    (&["circular"], tr_noop("Circular formulas")),
    (&["unboundColor", "unboundSize"], tr_noop("Not on a variable")),
    (&["unused"], tr_noop("Unused variables")),
    (&["duplicate"], tr_noop("Duplicate tokens")),
    (&["noScope"], tr_noop("Without a scope")),
];

/// The header's **Inspect** button (draw it in the editor's header bar).
pub(crate) fn button(ui: &mut Ui) -> Response {
    let resp = widgets::button(ui, tr("Inspect"), false, true).on_hover_text(tr("Check the design system: unbound values, broken references, unused variables…"));
    if resp.clicked() {
        let open = ui.data(|d| d.get_temp::<u64>(open_id())).is_some();
        let now = ui.ctx().cumulative_frame_nr();
        ui.data_mut(|d| {
            if open {
                d.remove_temp::<u64>(open_id());
            } else {
                d.insert_temp(open_id(), now);
            }
        });
        if open {
            crate::popover_ui::forget(open_id().with("area"));
        }
    }
    resp
}

/// The inspector popover beside `button`, while open.
pub(crate) fn draw(app: &mut FigApp, ctx: &egui::Context, button: &Response) {
    let Some(opened) = ctx.data(|d| d.get_temp::<u64>(open_id())) else { return };
    // The findings, recomputed when the file changes.
    let revision = app.session.ds().map(|d| d.revision).unwrap_or(0);
    let cache_id = open_id().with("cache");
    let cached: Option<(u64, Vec<Value>)> = ctx.data(|d| d.get_temp(cache_id));
    let findings = match cached {
        Some((r, f)) if r == revision => f,
        _ => {
            let f = app.run("designSystem.inspect", json!({})).and_then(|r| r["findings"].as_array().cloned()).unwrap_or_default();
            ctx.data_mut(|d| d.insert_temp(cache_id, (revision, f.clone())));
            f
        }
    };
    let t = Tokens::get(ctx);
    let mut fixes: Vec<Value> = Vec::new();
    let mut jump: Option<Value> = None;
    let at = pos2(button.rect.right() - W, button.rect.bottom() + 6.0);
    let title = trf("Design system · {n} findings", &[("n", &findings.len().to_string())]);
    let (closed, rect) = crate::popover_ui::show(ctx, open_id().with("area"), &title, W, crate::popover::Open::At(at), |ui| {
        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin { left: 16, right: 16, top: 10, bottom: 12 }).show(ui, |ui| {
                ui.set_width(W - 32.0);
                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                if findings.is_empty() {
                    ui.label(RichText::new(tr("Nothing to fix: every value is on a variable and every variable holds together.")).font(regular(11.0)).color(t.text_secondary));
                    return;
                }
                for (kinds, group) in GROUPS {
                    let list: Vec<&Value> = findings.iter().filter(|f| f["kind"].as_str().is_some_and(|k| kinds.contains(&k))).collect();
                    if list.is_empty() {
                        continue;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} · {}", tr(group), list.len())).font(semibold(11.0)).color(t.text));
                        let fixable: Vec<Value> = list.iter().filter_map(|f| f.get("fix").cloned()).collect();
                        if !fixable.is_empty() {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if widgets::button(ui, &trf("Fix all {n}", &[("n", &fixable.len().to_string())]), false, true).clicked() {
                                    fixes.extend(fixable);
                                }
                            });
                        }
                    });
                    for (i, f) in list.iter().enumerate().take(200) {
                        row(ui, &t, f, (group, i), &mut fixes, &mut jump);
                    }
                }
            });
        });
    });
    if !fixes.is_empty() {
        app.run("command.batch", json!({"label": "Fix design system", "commands": fixes}));
    }
    if let Some(f) = jump {
        go_to(app, &f);
    }
    let outside = ctx.cumulative_frame_nr() > opened + 1
        && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p) && !button.rect.contains(p)))
        && !egui::Popup::is_any_open(ctx);
    if closed || outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.data_mut(|d| d.remove_temp::<u64>(open_id()));
        crate::popover_ui::forget(open_id().with("area"));
    }
}

/// One finding: its text (a click jumps to it) and its Fix button.
fn row(ui: &mut Ui, t: &Tokens, f: &Value, key: (&str, usize), fixes: &mut Vec<Value>, jump: &mut Option<Value>) {
    let color = match f["severity"].as_str() {
        Some("error") => t.danger,
        Some("warning") => t.warning,
        _ => t.text_tertiary,
    };
    ui.horizontal(|ui| {
        let (dot, _) = ui.allocate_exact_size(vec2(8.0, 16.0), Sense::hover());
        ui.painter().circle_filled(dot.center(), 3.0, color);
        let fix_w = if f.get("fix").is_some() { 44.0 } else { 0.0 };
        let text = f["text"].as_str().unwrap_or_default();
        let resp = ui
            .push_id(key, |ui| {
                let w = ui.available_width() - fix_w;
                ui.allocate_ui_with_layout(vec2(w, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_width(w);
                    ui.add(egui::Label::new(RichText::new(text).font(regular(10.5)).color(t.text)).truncate().sense(Sense::click()))
                })
                .inner
            })
            .inner;
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.on_hover_text(tr("Go to it")).clicked() {
            *jump = Some(f.clone());
        }
        if let Some(fix) = f.get("fix")
            && ui.push_id((key, "fix"), |ui| widgets::button(ui, tr("Fix"), false, true)).inner.clicked()
        {
            fixes.push(fix.clone());
        }
    });
}

/// Jump to a finding's layer, else its variable.
fn go_to(app: &mut FigApp, f: &Value) {
    if let Some(id) = f["node"].as_u64() {
        if let Some(page) = app.session.doc().ok().and_then(|d| d.page_of(figcraft_doc::NodeId(id)))
            && app.session.doc().ok().and_then(|d| d.current_page) != Some(page)
        {
            app.run("page.select", json!({"id": page}));
        }
        app.run("select.set", json!({"ids": [id]}));
        app.run("view.zoomSelection", json!({}));
        return;
    }
    if let (Some(v), Some(c)) = (f["variable"].as_str(), f["collection"].as_str()) {
        app.ui.variables = Some(crate::variables_panel::Editor::on_variable(c.to_string(), v.to_string()));
    }
}
