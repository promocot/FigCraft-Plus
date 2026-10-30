//! FigCraft Plus: the colour picker's **Libraries** tab, beside **Custom**.
//!
//! It lists the colour variables (this file's, then each enabled library's) by collection and
//! group, with search and a library filter; a click applies one. Only variables whose scope fits
//! what the picker edits are offered: a stroke's picker lists the variables scoped to strokes, a
//! frame's fill those scoped to frame fills, a shadow's colour those scoped to effects. A colour
//! variable's value (Variables editor) offers the variables it can alias; a colour style's paint
//! this file's colour variables.

use egui::{Align2, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use figcraft_doc::VariableType;

use crate::FigApp;
use crate::colorpicker::PickerTarget;
use crate::i18n::tr;
use crate::theme::{Tokens, semibold};
use crate::variable_ui::Target;

/// Where the tab choice is kept (the picker reopens on the tab last used).
fn tab_id() -> Id {
    Id::unique("figcraftplus_picker_tab")
}

/// What picking a variable does for the picker's `target`; None where variables don't apply
/// (the page background, a plugin's field, a text range, several selection colours).
pub(crate) fn target_of(target: &PickerTarget) -> Option<Target> {
    Some(match target {
        PickerTarget::Paint { fills, index, ids } => {
            // FigCraft binds a first fill or stroke as "fill" / "stroke".
            let field = match (*fills, *index) {
                (true, 0) => "fill".to_string(),
                (false, 0) => "stroke".to_string(),
                (true, i) => format!("fills/{i}/color"),
                (false, i) => format!("strokes/{i}/color"),
            };
            Target::Bind { ids: ids.clone(), field }
        }
        PickerTarget::Effect { ids, index, secondary: false } => Target::Bind { ids: ids.clone(), field: format!("effects/{index}/color") },
        PickerTarget::Variable { id, mode } => Target::Alias { variable: id.clone(), mode: mode.clone() },
        PickerTarget::Style { id, index } => {
            Target::Style { style: id.clone(), field: if *index == 0 { "fill".to_string() } else { format!("fills/{index}/color") } }
        }
        _ => return None,
    })
}

/// The picker's header (`hr`): the Custom and Libraries tabs (Custom alone where variables don't
/// apply). True while Libraries is open.
pub(crate) fn header(ui: &mut Ui, hr: Rect, target: &PickerTarget) -> bool {
    let t = Tokens::get(ui.ctx());
    if target_of(target).is_none() {
        ui.painter().text(pos2(hr.min.x + 16.0, hr.center().y), Align2::LEFT_CENTER, tr("Custom"), semibold(11.0), t.text);
        crate::popover_ui::divider(ui, hr);
        return false;
    }
    let mut libraries = ui.data(|d| d.get_temp::<bool>(tab_id())).unwrap_or(false);
    let mut x = hr.min.x + 8.0;
    for (label, is_libraries) in [(tr("Custom"), false), (tr("Libraries"), true)] {
        let gw = ui.painter().layout_no_wrap(label.to_string(), semibold(11.0), t.text).size().x;
        let r = Rect::from_min_size(pos2(x, hr.center().y - 12.0), vec2(gw + 16.0, 24.0));
        let resp = ui.interact(r, tab_id().with(is_libraries), Sense::click());
        let on = libraries == is_libraries;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, label));
        if on || resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(6), t.hover);
        }
        ui.painter().text(r.center(), Align2::CENTER_CENTER, label, semibold(11.0), if on { t.text } else { t.text_secondary });
        if resp.clicked() && !on {
            libraries = is_libraries;
            ui.data_mut(|d| d.insert_temp(tab_id(), libraries));
        }
        x = r.max.x + 2.0;
    }
    crate::popover_ui::divider(ui, hr);
    libraries
}

/// The Libraries tab's list, `w` wide.
pub(crate) fn list(app: &mut FigApp, ui: &mut Ui, w: f32, target: &PickerTarget) {
    let Some(target) = target_of(target) else { return };
    let Ok(doc) = app.session.ds().map(|d| d.doc.clone()) else { return };
    let (items, current) = crate::var_picker_ui::offered(app, &doc, &target, Some(VariableType::Color));
    let empty = match &target {
        Target::Bind { .. } => tr("No colour variables are scoped to this property. Turn it on in a variable's scope (Variables ▸ Edit variable)."),
        _ => tr("No colour variables fit here."),
    };
    let id = Id::unique("figcraftplus_picker_libraries");
    if let Some(pick) = crate::var_picker_ui::embedded(ui, id, &items, current.as_ref(), empty, w).and_then(|i| items.get(i)).map(|it| it.pick.clone()) {
        crate::var_picker_ui::apply(app, &target, &pick);
    }
}
