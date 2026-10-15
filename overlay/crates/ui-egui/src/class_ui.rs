//! FigCraft Plus: layer classes and HTML tags in the Design panel (see the engine's
//! `cmd::layer_classes`).
//!
//! - The selection's "Class" section: its HTML tag, its class (pick one, or create one from the
//!   layer), the sections it overrides with Reset and "Update class", and a menu to edit, rename or
//!   detach the class.
//! - With nothing selected, "Local classes" under Local styles: every class with how many layers
//!   use it; a click selects them, right-click renames or deletes.

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use figcraft_doc::{Document, NodeId, NodeKind};
use figcraft_engine::cmd::layer_classes::{self as lc, LayerClass};
use serde_json::json;

use crate::FigApp;
use crate::a11y::Tip;
use crate::i18n::{tr, trf};
use crate::icons::{self, Icon};
use crate::right::{context_popup, menu_check, menu_item, menu_popup, row};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets;

/// A section's label.
fn section_label(s: &str) -> &'static str {
    match s {
        "layout" => tr("Layout"),
        "appearance" => tr("Appearance"),
        "fill" => tr("Fill"),
        "stroke" => tr("Stroke"),
        "effects" => tr("Effects"),
        "typography" => tr("Typography"),
        _ => "",
    }
}

/// The class dialog: create a class from a layer, or edit one (name and sections, values taken
/// from the layer).
#[derive(Clone, Debug, Default)]
struct Dialog {
    node: Option<NodeId>,
    /// None: a new class.
    class: Option<String>,
    name: String,
    sections: Vec<String>,
    focus: bool,
}

fn dialog_id() -> Id {
    Id::new("figcraftplus_class_dialog")
}

/// The Design panel's Class section for the selection (with its separator).
pub(crate) fn section(app: &mut FigApp, ui: &mut Ui, doc: &Document, sel: &[NodeId]) {
    let nodes: Vec<&figcraft_doc::Node> = sel.iter().filter_map(|i| doc.node(*i)).collect();
    if nodes.is_empty() || nodes.iter().any(|n| n.is_page()) {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let list = lc::classes(doc);
    let ids = sel.to_vec();
    let texts = nodes.iter().all(|n| matches!(n.kind, NodeKind::Text(_)));
    let others = nodes.iter().all(|n| !matches!(n.kind, NodeKind::Text(_)));
    let common = |f: &dyn Fn(&figcraft_doc::Node) -> Option<String>| -> Option<Option<String>> {
        let first = f(nodes.first()?);
        nodes.iter().all(|n| f(n) == first).then_some(first)
    };
    let class_id = common(&|n| lc::class_of(n).map(String::from));
    let class = class_id.clone().flatten().and_then(|k| lc::find(&list, &k).cloned());
    // A small label, then one row: the tag, the class and its menu.
    ui.add_space(6.0);
    widgets::sublabels(ui, tr("Class"), None);
    let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let ctrl = Rect::from_min_max(pos2(hr.min.x + widgets::PAD, hr.min.y + 4.0), pos2(hr.max.x - 8.0, hr.max.y - 4.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(ctrl).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.spacing_mut().item_spacing = vec2(8.0, 0.0);
    {
        let ui = &mut child;
        let tag_w = 76.0;
        let class_w = (ctrl.width() - tag_w - 8.0 - 32.0).max(80.0);
        // The HTML tag (texts and the rest have their own lists).
        if texts || others {
            let tags: &[&str] = if texts { &lc::TEXT_TAGS } else { &lc::FRAME_TAGS };
            let tag = common(&|n| Some(lc::tag_of(n).to_string())).flatten();
            let label = tag.clone().unwrap_or_else(|| tr("Mixed").to_string());
            let resp = widgets::dropdown(ui, &label, tag_w, Some(Icon::Code)).tip(tr("HTML tag"));
            menu_popup(&resp).width(140.0).show(|ui| {
                for tg in tags {
                    if menu_check(ui, tag.as_deref() == Some(*tg), tg, "") {
                        app.run("layer.setTag", json!({"ids": ids, "tag": tg}));
                        ui.close();
                    }
                }
            });
        }
        // The class.
        let label = match (&class_id, &class) {
            (None, _) => tr("Mixed").to_string(),
            (Some(None), _) => tr("No class").to_string(),
            (_, Some(c)) => c.name.clone(),
            (Some(Some(_)), None) => tr("No class").to_string(),
        };
        let resp = widgets::dropdown(ui, &label, class_w, Some(Icon::Styles)).tip(tr("Class"));
        menu_popup(&resp).width(220.0).show(|ui| {
            if menu_check(ui, class.is_none() && class_id == Some(None), tr("No class"), "") {
                app.run("class.detach", json!({"ids": ids}));
                ui.close();
            }
            if !list.is_empty() {
                ui.separator();
            }
            for c in &list {
                if menu_check(ui, class.as_ref().is_some_and(|x| x.id == c.id), &c.name, "") {
                    app.run("class.apply", json!({"ids": ids, "class": c.id}));
                    ui.close();
                }
            }
            if ids.len() == 1 {
                ui.separator();
                if menu_item(ui, tr("Create class from layer…"), "") {
                    if let Some(n) = nodes.first() {
                        open_dialog(ui.ctx(), Dialog {
                            node: Some(n.id),
                            class: None,
                            name: String::new(),
                            sections: lc::default_sections(n),
                            focus: true,
                        });
                    }
                    ui.close();
                }
            }
        });
        // The class's menu.
        if let Some(c) = &class {
            let mresp = icons::button(ui, Icon::Dots, 24.0, false, tr("Class options"));
            menu_popup(&mresp).width(240.0).show(|ui| {
                if menu_item(ui, tr("Reset to class"), "") {
                    app.run("class.reset", json!({"ids": ids}));
                    ui.close();
                }
                if ids.len() == 1 {
                    if menu_item(ui, tr("Update class from this layer"), "") {
                        app.run("class.update", json!({"class": c.id, "from": ids.first()}));
                        ui.close();
                    }
                    if menu_item(ui, tr("Edit class…"), "") {
                        open_dialog(ui.ctx(), Dialog {
                            node: ids.first().copied(),
                            class: Some(c.id.clone()),
                            name: c.name.clone(),
                            sections: c.sections.clone(),
                            focus: false,
                        });
                        ui.close();
                    }
                }
                if menu_item(ui, tr("Detach class"), "") {
                    app.run("class.detach", json!({"ids": ids}));
                    ui.close();
                }
            });
        }
    }
    // What the layer overrides, with Reset and Update class.
    if let Some(c) = &class {
        let over: Vec<String> = {
            let mut all: Vec<String> = Vec::new();
            for n in &nodes {
                for s in lc::overrides(n, c) {
                    if !all.contains(&s) {
                        all.push(s);
                    }
                }
            }
            all
        };
        let applies: Vec<&str> = c.sections.iter().filter(|s| nodes.iter().any(|n| lc::applies(n, s))).map(|s| section_label(s)).collect();
        row(ui, 24.0, |ui| {
            let text = if over.is_empty() {
                applies.join(" · ")
            } else {
                let list: Vec<&str> = over.iter().map(|s| section_label(s)).collect();
                trf("Overrides: {list}", &[("list", &list.join(", "))])
            };
            let w = ui.available_width() - if over.is_empty() { 0.0 } else { 60.0 };
            let shown = widgets::elide_font(ui, &text, w, regular(10.0));
            let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
            let colour = if over.is_empty() { t.text_secondary } else { t.accent };
            ui.painter().text(r.left_center(), Align2::LEFT_CENTER, shown, regular(10.0), colour);
            if !over.is_empty() {
                if icons::button(ui, Icon::Reset, 22.0, false, tr("Reset to class")).clicked() {
                    app.run("class.reset", json!({"ids": ids}));
                }
                if ids.len() == 1 && icons::button(ui, Icon::Export, 22.0, false, tr("Update class from this layer")).clicked() {
                    app.run("class.update", json!({"class": c.id, "from": ids.first()}));
                }
            }
        });
    }
    widgets::separator(ui);
    dialog(app, ui.ctx(), doc, &list);
}

fn open_dialog(ctx: &egui::Context, d: Dialog) {
    ctx.data_mut(|m| m.insert_temp(dialog_id(), d));
}

/// The class dialog (beside the Design panel) while it is open.
fn dialog(app: &mut FigApp, ctx: &egui::Context, doc: &Document, list: &[LayerClass]) {
    let Some(mut d) = ctx.data(|m| m.get_temp::<Dialog>(dialog_id())) else { return };
    let Some(n) = d.node.and_then(|i| doc.node(i)).cloned() else {
        ctx.data_mut(|m| m.remove::<Dialog>(dialog_id()));
        return;
    };
    let t = Tokens::get(ctx);
    const W: f32 = 280.0;
    let screen = ctx.content_rect();
    let x = (screen.max.x - app.ui.right_width - W - 16.0).max(screen.min.x + 8.0);
    let y = screen.min.y + 120.0;
    let mut close = false;
    let mut ok = false;
    let title = if d.class.is_some() { tr("Edit class") } else { tr("Create class") };
    let area = egui::Area::new(dialog_id().with("area")).order(egui::Order::Foreground).fixed_pos(pos2(x, y)).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(t.panel)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, t.border))
            .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: t.shadow })
            .show(ui, |ui| {
                ui.set_width(W);
                ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                let (hr, _) = ui.allocate_exact_size(vec2(W, 44.0), Sense::hover());
                ui.painter().text(pos2(hr.min.x + 16.0, hr.center().y), Align2::LEFT_CENTER, title, semibold(12.0), t.text);
                let cr = Rect::from_center_size(pos2(hr.max.x - 22.0, hr.center().y), vec2(24.0, 24.0));
                let cresp = ui.interact(cr, dialog_id().with("close"), Sense::click()).tip(tr("Close"));
                if cresp.hovered() {
                    ui.painter().rect_filled(cr, CornerRadius::same(5), t.hover);
                }
                icons::paint(ui.painter(), cr.shrink(6.0), Icon::Close, t.icon);
                close |= cresp.clicked();
                ui.painter().hline(hr.x_range(), hr.max.y - 0.5, Stroke::new(1.0, t.border));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut d.name).hint_text(tr("Class name, e.g. card or card__title")).font(regular(11.0)).desired_width(W - 32.0),
                    );
                    if d.focus {
                        r.request_focus();
                        d.focus = false;
                    }
                    ok |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    widgets::label(ui, tr("Sections"), t.text_secondary);
                });
                for s in lc::SECTIONS.iter().filter(|s| lc::applies(&n, s)) {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let on = d.sections.iter().any(|x| x == s);
                        if widgets::checkbox(ui, on, section_label(s)) {
                            if on {
                                d.sections.retain(|x| x != s);
                            } else {
                                d.sections.push(s.to_string());
                            }
                        }
                    });
                }
                // Sections this layer can't give, kept from the class.
                for s in d.sections.clone().iter().filter(|s| !lc::applies(&n, s)) {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        widgets::label(ui, &trf("{section} (kept)", &[("section", &section_label(s))]), t.text_tertiary);
                    });
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let hint = tr("The values come from the selected layer.");
                    ui.label(egui::RichText::new(hint).font(regular(10.0)).color(t.text_secondary));
                });
                ui.add_space(4.0);
                ui.painter().hline(ui.min_rect().x_range(), ui.min_rect().max.y + 0.5, Stroke::new(1.0, t.border));
                let (fr, _) = ui.allocate_exact_size(vec2(W, 48.0), Sense::hover());
                let br = Rect::from_min_max(pos2(fr.max.x - 120.0, fr.min.y + 10.0), pos2(fr.max.x - 14.0, fr.max.y - 10.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(br));
                let can = !d.name.trim().is_empty() && !d.sections.is_empty();
                let label = if d.class.is_some() { tr("Save") } else { tr("Create class") };
                ok |= widgets::button(&mut child, label, true, can).clicked();
                ok &= can;
            });
    });
    let rect = area.response.rect;
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if ok {
        let name = d.name.trim().to_string();
        let sections = lc::SECTIONS.iter().filter(|s| d.sections.iter().any(|x| x == *s)).copied().collect::<Vec<_>>();
        match &d.class {
            None => {
                app.run("class.create", json!({"id": n.id, "name": name, "sections": sections}));
            }
            Some(cid) => {
                let mut steps = Vec::new();
                if lc::find(list, cid).is_some_and(|c| c.name != name) {
                    steps.push(json!({"command": "class.rename", "params": {"class": cid, "name": name}}));
                }
                steps.push(json!({"command": "class.update", "params": {"class": cid, "from": n.id, "sections": sections}}));
                app.run("command.batch", json!({"label": "Edit class", "commands": steps}));
            }
        }
        close = true;
    }
    let _ = rect;
    if close {
        ctx.data_mut(|m| m.remove::<Dialog>(dialog_id()));
    } else {
        ctx.data_mut(|m| m.insert_temp(dialog_id(), d));
    }
}

/// With nothing selected: the file's classes, under Local styles.
pub(crate) fn local_classes(app: &mut FigApp, ui: &mut Ui, doc: &Document) {
    let list = lc::classes(doc);
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Local classes"), &[], list.is_empty());
    if list.is_empty() {
        row(ui, 28.0, |ui| {
            ui.label(egui::RichText::new(tr("Select a layer and choose Class ▸ Create class from layer…")).font(regular(10.0)).color(t.text_secondary));
        });
        return;
    }
    let renaming: Option<String> = ui.data(|d| d.get_temp(Id::new("figcraftplus_class_rename")));
    let mut run: Option<(&str, serde_json::Value)> = None;
    for c in &list {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::ZERO, t.hover);
        }
        let x = r.min.x + widgets::PAD;
        icons::paint(ui.painter(), Rect::from_center_size(pos2(x + 8.0, r.center().y), vec2(14.0, 14.0)), Icon::Styles, t.icon_secondary);
        let members = lc::members(doc, &c.id);
        let count = members.len().to_string();
        let cw = ui.painter().text(pos2(r.max.x - widgets::PAD, r.center().y), Align2::RIGHT_CENTER, &count, regular(10.0), t.text_secondary).width();
        let name_r = Rect::from_min_max(pos2(x + 24.0, r.min.y + 4.0), pos2(r.max.x - widgets::PAD - cw - 8.0, r.max.y - 4.0));
        if renaming.as_deref() == Some(c.id.as_str()) {
            let rid = Id::new(("figcraftplus_class_rename_field", &c.id));
            let start = ui.data(|d| d.get_temp::<bool>(rid.with("started"))).is_none();
            let (out, open) = widgets::rename_field(ui, rid, &c.name, name_r, regular(11.0), start);
            ui.data_mut(|d| d.insert_temp(rid.with("started"), true));
            if let Some(name) = out.filter(|n| !n.trim().is_empty() && n.trim() != c.name) {
                run = Some(("class.rename", json!({"class": c.id, "name": name.trim()})));
            }
            if !open {
                ui.data_mut(|d| {
                    d.remove::<String>(Id::new("figcraftplus_class_rename"));
                    d.remove::<bool>(rid.with("started"));
                });
            }
        } else {
            let shown = widgets::elide_font(ui, &c.name, name_r.width(), regular(11.0));
            ui.painter().text(name_r.left_center(), Align2::LEFT_CENTER, shown, regular(11.0), t.text);
        }
        let sections: Vec<&str> = c.sections.iter().map(|s| section_label(s)).collect();
        let resp = resp.tip(format!("{}\n{}", c.name, sections.join(" · ")));
        if resp.double_clicked() {
            ui.data_mut(|d| d.insert_temp(Id::new("figcraftplus_class_rename"), c.id.clone()));
        } else if resp.clicked() && !members.is_empty() {
            run = Some(("select.set", json!({"ids": members})));
        }
        context_popup(&resp).show(|ui| {
            if menu_item(ui, tr("Select layers"), "") {
                run = Some(("select.set", json!({"ids": members})));
                ui.close();
            }
            if menu_item(ui, tr("Rename"), "") {
                ui.data_mut(|d| d.insert_temp(Id::new("figcraftplus_class_rename"), c.id.clone()));
                ui.close();
            }
            if menu_item(ui, tr("Delete"), "") {
                run = Some(("class.delete", json!({"class": c.id})));
                ui.close();
            }
        });
    }
    if let Some((cmd, p)) = run {
        app.run(cmd, p);
    }
}
