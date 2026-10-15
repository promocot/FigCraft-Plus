//! FigCraft Plus: variables on the text section's font family, font weight / style and paragraph
//! spacing (as in Figma), and the Variables editor's "Edit variable" button (details and scope)
//! on every row.
//!
//! The font style dropdown takes either a number variable (the weight: 400, 700) or a string one
//! (the style: "Bold", "SemiBold Italic"); the picker lists both, and binding one replaces the
//! other in one undo step.

use egui::{CornerRadius, Rect, Sense, Ui, pos2, vec2};
use figcraft_doc::{NodeId, Variable, VariableType};
use serde_json::json;

use crate::FigApp;
use crate::a11y::Tip;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::Tokens;
use crate::variable_ui::{self, Pick};
use crate::widgets;

/// The picker field of the font style dropdown: a weight or a style variable.
pub(crate) const WEIGHT_OR_STYLE: &str = "fontWeight|fontStyle";

/// Whether the font style dropdown's picker offers `v`: number variables scoped to font weight,
/// string variables scoped to font style.
pub(crate) fn weight_fits(v: &Variable) -> bool {
    match v.kind {
        VariableType::Number => v.in_scope("FONT_WEIGHT"),
        VariableType::String => v.in_scope("FONT_STYLE"),
        _ => false,
    }
}

/// The field a variable of `kind` binds to from the font style dropdown, and the other one.
fn weight_fields(kind: VariableType) -> (&'static str, &'static str) {
    if kind == VariableType::Number { ("fontWeight", "fontStyle") } else { ("fontStyle", "fontWeight") }
}

/// The type of the variable `pick` names (this file's or an enabled library's).
fn pick_kind(app: &FigApp, pick: &Pick) -> Option<VariableType> {
    let doc = app.session.doc().ok()?;
    match &pick.library {
        None => doc.variables.get(&pick.id).map(|v| v.kind),
        Some(lib) => {
            variable_ui::libraries(app, doc).iter().find(|l| &l.manifest.id == lib).and_then(|l| l.doc.variables.get(&pick.id).map(|v| v.kind))
        }
    }
}

/// Bind a variable picked from the font style dropdown: a number one to the weight, a string one
/// to the style, unbinding the other in the same undo step. False when `field` isn't that
/// dropdown's (the caller binds as usual).
pub(crate) fn bind_weight_or_style(app: &mut FigApp, ids: &[NodeId], field: &str, pick: &Pick) -> bool {
    if field != WEIGHT_OR_STYLE {
        return false;
    }
    let Some(kind) = pick_kind(app, pick) else { return true };
    let (field, other) = weight_fields(kind);
    let mut steps = Vec::new();
    let bound_other = app.session.doc().ok().is_some_and(|d| ids.iter().any(|i| d.node(*i).is_some_and(|n| n.bound.contains_key(other))));
    if bound_other {
        steps.push(json!({"command": "variable.bind", "params": {"ids": ids, "field": other, "variable": null}}));
    }
    steps.push(match &pick.library {
        Some(lib) => json!({"command": "library.bindVariable", "params": {"library": lib, "variable": pick.id, "ids": ids, "field": field}}),
        None => json!({"command": "variable.bind", "params": {"ids": ids, "field": field, "variable": pick.id}}),
    });
    app.run("command.batch", json!({"label": "Apply variable", "commands": steps}));
    true
}

/// Bind a variable picked for `field` of the shared style `style`: from a text style's style
/// dropdown ([`WEIGHT_OR_STYLE`]) a number binds the weight and a string the style, unbinding the
/// other in the same undo step; any other field binds as usual.
pub(crate) fn bind_style(app: &mut FigApp, style: &str, field: &str, pick: &Pick) {
    if field != WEIGHT_OR_STYLE {
        app.run("style.bind", json!({"style": style, "field": field, "variable": pick.id}));
        return;
    }
    let Some(kind) = pick_kind(app, pick) else { return };
    let (field, other) = weight_fields(kind);
    let mut steps = Vec::new();
    if app.session.doc().ok().and_then(|d| d.style(style)).is_some_and(|s| s.bound.contains_key(other)) {
        steps.push(json!({"command": "style.bind", "params": {"style": style, "field": other, "variable": null}}));
    }
    steps.push(json!({"command": "style.bind", "params": {"style": style, "field": field, "variable": pick.id}}));
    app.run("command.batch", json!({"label": "Apply variable", "commands": steps}));
}

/// The font family dropdown (`control`, `width` wide) with a variable button on hover, or the
/// bound string variable's chip. While text is being edited it is just the dropdown.
pub(crate) fn family(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], editing: bool, width: f32, control: impl FnOnce(&mut FigApp, &mut Ui)) {
    if editing {
        control(app, ui);
    } else {
        variable_ui::bindable(app, ui, ids, "fontFamily", VariableType::String, width, control);
    }
}

/// The font style dropdown (`control`, `width` wide) with a variable button on hover that offers
/// weight (number) and style (string) variables, or the bound one's chip.
pub(crate) fn weight_or_style(app: &mut FigApp, ui: &mut Ui, ids: &[NodeId], editing: bool, width: f32, control: impl FnOnce(&mut FigApp, &mut Ui)) {
    if editing {
        control(app, ui);
        return;
    }
    let area = Rect::from_min_size(ui.cursor().min, vec2(width, widgets::FIELD_H));
    let bound = app.session.doc().ok().and_then(|d| {
        ["fontWeight", "fontStyle"].into_iter().find_map(|f| variable_ui::bound_name(d, ids, f).map(|name| (f, name)))
    });
    if let Some((field, name)) = bound {
        // The chip changes to any weight or style variable, or detaches this one.
        let (r, _) = ui.allocate_exact_size(area.size(), Sense::hover());
        match widgets::bound_field(ui, egui::Id::new(("bound_chip", WEIGHT_OR_STYLE, ids)), widgets::Lead::None, r, &name) {
            Some(widgets::BindEvent::Open(at)) => variable_ui::open(ui, ids, WEIGHT_OR_STYLE, VariableType::Number, at),
            Some(widgets::BindEvent::Detach) => {
                app.run("variable.bind", json!({"ids": ids, "field": field, "variable": null}));
            }
            None => {}
        }
        return;
    }
    control(app, ui);
    if variable_ui::variable_button(ui, area, egui::Id::new(("bind_button", WEIGHT_OR_STYLE, ids))) {
        variable_ui::open(ui, ids, WEIGHT_OR_STYLE, VariableType::Number, area.left_bottom());
    }
}

/// The "Edit variable" button at the end of a Variables editor row's name cell (`name_w` wide):
/// shown while the pointer is on the row or its details are open (`open`). True when clicked.
pub(crate) fn details_button(ui: &mut Ui, row: Rect, name_w: f32, open: bool) -> bool {
    if open {
        // The open row anchors the Edit variable window (it follows scrolling and the editor).
        let editor = ui.ctx().memory(|m| m.area_rect(ui.layer_id().id));
        ui.ctx().data_mut(|d| d.insert_temp(details_anchor_id(), (row, editor)));
    }
    if !open && !ui.rect_contains_pointer(row) {
        return false;
    }
    let t = Tokens::get(ui.ctx());
    let b = Rect::from_center_size(pos2(row.min.x + name_w - 18.0, row.center().y), vec2(22.0, 22.0));
    let resp = ui.interact(b, ui.id().with(("var_details_button", row.min.y as i32)), Sense::click()).tip(tr("Edit variable"));
    ui.painter().rect_filled(b, CornerRadius::same(4), if resp.hovered() || open { t.field } else { t.hover });
    icons::paint(ui.painter(), b.shrink(4.0), Icon::Adjust, if open { t.accent } else { t.icon_secondary });
    resp.clicked()
}

fn details_anchor_id() -> egui::Id {
    egui::Id::new("figcraftplus_details_anchor")
}

/// Where the Edit variable window goes, as in Figma: beside the Variables editor (right of it, else
/// left, else over its right end) at the height of the variable's row, inside the window. It is
/// drawn above the editor.
pub(crate) fn details_pos(ctx: &egui::Context) -> egui::Pos2 {
    let screen = ctx.content_rect();
    let size = ctx.memory(|m| m.area_rect(egui::Id::new("variable_details"))).map_or(vec2(312.0, 600.0), |r| r.size());
    let anchor: Option<(Rect, Option<Rect>)> = ctx.data(|d| d.get_temp(details_anchor_id()));
    let Some((row, editor)) = anchor else { return pos2(screen.center().x - size.x / 2.0, screen.min.y + 80.0) };
    let editor = editor.unwrap_or(row);
    let x = if editor.max.x + 8.0 + size.x <= screen.max.x - 8.0 {
        editor.max.x + 8.0
    } else if editor.min.x - 8.0 - size.x >= screen.min.x + 8.0 {
        editor.min.x - 8.0 - size.x
    } else {
        editor.max.x - size.x - 8.0
    };
    let y = (row.min.y - 40.0).min(screen.max.y - size.y - 8.0).max(screen.min.y + 8.0);
    pos2(x.max(screen.min.x + 8.0), y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use figcraft_doc::VariableType;

    fn var(kind: VariableType, scopes: Option<Vec<&str>>) -> Variable {
        Variable {
            id: "VariableID:1".into(),
            name: "v".into(),
            collection: "c".into(),
            kind,
            values: Default::default(),
            description: String::new(),
            scopes: scopes.map(|s| s.into_iter().map(String::from).collect()),
            hidden_from_publishing: false,
            extends: None,
            overrides: Default::default(),
            code_syntax: Default::default(),
        }
    }

    #[test]
    fn the_style_dropdown_offers_weights_and_styles_in_scope() {
        assert!(weight_fits(&var(VariableType::Number, None)));
        assert!(weight_fits(&var(VariableType::String, None)));
        assert!(!weight_fits(&var(VariableType::Color, None)));
        assert!(!weight_fits(&var(VariableType::Boolean, None)));
        assert!(weight_fits(&var(VariableType::Number, Some(vec!["FONT_WEIGHT"]))));
        assert!(!weight_fits(&var(VariableType::Number, Some(vec!["FONT_SIZE"]))));
        assert!(weight_fits(&var(VariableType::String, Some(vec!["FONT_STYLE"]))));
        assert!(!weight_fits(&var(VariableType::String, Some(vec!["FONT_FAMILY"]))));
        assert_eq!(weight_fields(VariableType::Number), ("fontWeight", "fontStyle"));
        assert_eq!(weight_fields(VariableType::String), ("fontStyle", "fontWeight"));
    }
}

