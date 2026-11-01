//! FigCraft Plus: responsive layout.
//!
//! **Relative sizes.** A layer's width or height can be a share of its parent — `50%` — or of the
//! frame it sits in on the canvas (its viewport) — `100vw`, `100vh`. The engine keeps the size in
//! step after every edit: resize the parent (or the viewport frame) and the layer follows, its
//! children by their constraints. The size itself stays plain px in the file; the rule is a line of
//! layer data (`figcraftplus.layout`), which the official FigCraft keeps and ignores:
//!
//! ```text
//! width 50%        half the parent's width (inside its padding when the parent is auto layout)
//! height 100vh     the viewport frame's height
//! ```
//!
//! Resizing the layer by hand (W / H, its handles) ends its rule for that side, as setting a fixed
//! size does in CSS.
//!
//! **Auto layout from variables.** An auto layout frame's direction, wrap and alignment, and a
//! layer's Hug / Fill / Fixed sizing, can follow a string (or boolean) variable, so a mode switches
//! them — a card that is a row on Desktop and a column on Mobile:
//!
//! ```text
//! direction {Layout/Card direction}    row | column | horizontal | vertical | grid
//! wrap {Layout/Wrap}                   true / false, or wrap / nowrap
//! mainAlign {…}                        start | center | end | space-between | space-around | space-evenly
//! crossAlign {…}                       start | center | end | baseline
//! sizingH {…}  sizingV {…}             fixed | hug | fill
//! ```
//!
//! The variable is read in the modes that apply to the layer (its frames' modes).

use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::{CrossAlign, Document, LayoutMode, MainAlign, NodeId, Sizing, VariableValue};
use serde_json::Value;

use crate::cmd::{CommandSpec, ok};
use crate::params::{get, req_string, string};
use crate::{Result, Session, bad, spec};

/// The layer data key holding a layer's responsive rules, one per line.
pub const KEY: &str = "figcraftplus.layout";

/// A relative size's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// A share of the parent (its inner box when it is auto layout).
    Percent,
    /// A share of the viewport frame's width.
    Vw,
    /// A share of the viewport frame's height.
    Vh,
}

impl Unit {
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Percent => "%",
            Unit::Vw => "vw",
            Unit::Vh => "vh",
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        match s.trim().to_ascii_lowercase().as_str() {
            "%" => Some(Unit::Percent),
            "vw" => Some(Unit::Vw),
            "vh" => Some(Unit::Vh),
            _ => None,
        }
    }
}

/// What one line says.
#[derive(Clone, Debug, PartialEq)]
pub enum Rule {
    /// `width 50%` (`horizontal`) or `height 100vh`.
    Size { horizontal: bool, value: f64, unit: Unit },
    /// `direction {Name}` etc.: a layout field following a variable.
    Var { field: String, variable: String },
}

/// The layout fields a variable can drive.
pub const VAR_FIELDS: [&str; 6] = ["direction", "wrap", "mainAlign", "crossAlign", "sizingH", "sizingV"];

pub fn parse_line(line: &str) -> Option<Rule> {
    let (head, rest) = line.trim().split_once(char::is_whitespace)?;
    let rest = rest.trim();
    match head {
        "width" | "height" => {
            let split = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))?;
            let value: f64 = rest[..split].parse().ok().filter(|v: &f64| v.is_finite())?;
            let unit = Unit::parse(&rest[split..])?;
            Some(Rule::Size { horizontal: head == "width", value, unit })
        }
        f if VAR_FIELDS.contains(&f) => {
            let name = rest.strip_prefix('{')?.strip_suffix('}')?.trim();
            (!name.is_empty()).then(|| Rule::Var { field: f.to_string(), variable: name.to_string() })
        }
        _ => None,
    }
}

pub fn format(rule: &Rule) -> String {
    match rule {
        Rule::Size { horizontal, value, unit } => {
            format!("{} {}{}", if *horizontal { "width" } else { "height" }, (value * 1e4).round() / 1e4, unit.suffix())
        }
        Rule::Var { field, variable } => format!("{field} {{{variable}}}"),
    }
}

/// Every rule of node `id`.
pub fn rules(doc: &Document, id: NodeId) -> Vec<Rule> {
    doc.node(id).and_then(|n| n.plugin_data.get(KEY)).map(|t| t.lines().filter_map(parse_line).collect()).unwrap_or_default()
}

/// The relative size rule of one side of `id`.
pub fn size_rule(doc: &Document, id: NodeId, horizontal: bool) -> Option<(f64, Unit)> {
    rules(doc, id).into_iter().find_map(|r| match r {
        Rule::Size { horizontal: h, value, unit } if h == horizontal => Some((value, unit)),
        _ => None,
    })
}

/// The variable driving layout field `field` of `id`.
pub fn var_rule(doc: &Document, id: NodeId, field: &str) -> Option<String> {
    rules(doc, id).into_iter().find_map(|r| match r {
        Rule::Var { field: f, variable } if f == field => Some(variable),
        _ => None,
    })
}

/// `text` with the rules matching `same` replaced by `new` (removed for None).
fn with_rule(text: &str, same: impl Fn(&Rule) -> bool, new: Option<&Rule>) -> String {
    let mut lines: Vec<String> = text.lines().filter(|l| !parse_line(l).is_some_and(|r| same(&r))).map(str::to_string).collect();
    if let Some(r) = new {
        lines.push(format(r));
    }
    lines.join("\n")
}

/// The box a relative size of `id` is a share of: (width, height) in px.
pub fn reference(doc: &Document, id: NodeId, unit: Unit) -> Option<(f64, f64)> {
    match unit {
        Unit::Percent => {
            let p = doc.node(doc.node(id)?.parent?)?;
            if p.is_page() {
                return None;
            }
            let (w, h) = (p.width, p.height);
            Some(match &p.layout {
                Some(l) => ((w - l.padding[1] - l.padding[3]).max(0.0), (h - l.padding[0] - l.padding[2]).max(0.0)),
                None => (w, h),
            })
        }
        Unit::Vw | Unit::Vh => {
            let top = doc.top_level_frame(id).filter(|t| *t != id)?;
            let n = doc.node(top)?;
            Some((n.width, n.height))
        }
    }
}

/// The px size a rule asks for.
pub fn px(doc: &Document, id: NodeId, horizontal: bool, value: f64, unit: Unit) -> Option<f64> {
    let (w, h) = reference(doc, id, unit)?;
    let base = match unit {
        Unit::Percent => {
            if horizontal {
                w
            } else {
                h
            }
        }
        Unit::Vw => w,
        Unit::Vh => h,
    };
    Some(base * value / 100.0)
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

/// A string (or boolean) variable's value for `id`, in the modes that apply there.
fn text_value(doc: &Document, id: NodeId, name: &str) -> Option<String> {
    let v = doc.variables.get(name)?;
    let modes = doc.modes_for(id);
    match doc.variables.resolve(&v.id, &modes)? {
        VariableValue::String(s) => Some(s.trim().to_ascii_lowercase()),
        VariableValue::Boolean(b) => Some(b.to_string()),
        VariableValue::Number(x) => Some(figcraft_doc::trim_num(x)),
        _ => None,
    }
}

fn sizing_of(s: &str) -> Option<Sizing> {
    Some(match s {
        "fixed" => Sizing::Fixed,
        "hug" | "auto" | "hug contents" | "fit-content" => Sizing::Hug,
        "fill" | "fill container" | "stretch" | "100%" => Sizing::Fill,
        _ => return None,
    })
}

/// Keep every responsive rule in step (called by the engine's normalisation, inside the edit's
/// undo step).
pub fn resolve(doc: &mut Document, before: &Document, touched: &BTreeSet<NodeId>) {
    let mut ids: Vec<NodeId> = doc.ids().filter(|id| doc.node(*id).is_some_and(|n| n.plugin_data.contains_key(KEY))).collect();
    if ids.is_empty() {
        return;
    }
    // Parents before children: a share of a share follows its parent's new size.
    ids.sort_by_key(|id| doc.ancestors(*id).len());
    let revalued = doc.variables != before.variables;
    for id in ids {
        let rs = rules(doc, id);
        // Layout from variables.
        if revalued || touched.contains(&id) || doc.ancestors(id).iter().any(|a| touched.contains(a)) {
            apply_vars(doc, id, &rs);
        }
        // Relative sizes.
        for r in &rs {
            let Rule::Size { horizontal, value, unit } = *r else { continue };
            let Some(want) = px(doc, id, horizontal, value, unit) else { continue };
            let Some(n) = doc.node(id) else { continue };
            let now = if horizontal { n.width } else { n.height };
            // Edited by hand: the size changed, what it is a share of didn't, and it held before.
            let was = before.node(id).map(|b| if horizontal { b.width } else { b.height });
            let held_before = before.node(id).is_some() && px(before, id, horizontal, value, unit).is_some_and(|b| was.is_some_and(|w| same(w, b)));
            let ref_same = reference(before, id, unit) == reference(doc, id, unit);
            let rule_was = before.node(id).and_then(|b| b.plugin_data.get(KEY)).is_some_and(|t| t.lines().filter_map(parse_line).any(|x| x == *r));
            if rule_was && held_before && ref_same && was.is_some_and(|w| !same(w, now)) {
                drop_rule(doc, id, horizontal);
                continue;
            }
            if same(now, want) {
                continue;
            }
            let (w, h) = if horizontal { (want, n.height) } else { (n.width, want) };
            // A relative side is a fixed size to auto layout (as `width: 50%` is in CSS).
            if let Some(n) = doc.node_mut(id) {
                if horizontal {
                    n.child_layout.sizing_h = Sizing::Fixed;
                } else {
                    n.child_layout.sizing_v = Sizing::Fixed;
                }
            }
            let _ = crate::cmd::props::resize_node(doc, id, w, h);
            doc.mark_dirty([id]);
        }
    }
}

fn drop_rule(doc: &mut Document, id: NodeId, horizontal: bool) {
    let Some(n) = doc.node_mut(id) else { return };
    let text = n.plugin_data.get(KEY).cloned().unwrap_or_default();
    let new = with_rule(&text, |r| matches!(r, Rule::Size { horizontal: h, .. } if *h == horizontal), None);
    if new.trim().is_empty() {
        n.plugin_data.remove(KEY);
    } else {
        n.plugin_data.insert(KEY.into(), new);
    }
}

fn apply_vars(doc: &mut Document, id: NodeId, rs: &[Rule]) {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    for r in rs {
        if let Rule::Var { field, variable } = r
            && let Some(v) = text_value(doc, id, variable)
        {
            values.insert(field.clone(), v);
        }
    }
    if values.is_empty() {
        return;
    }
    let Some(n) = doc.node_mut(id) else { return };
    let before = (n.layout.clone(), n.child_layout.clone());
    if let Some(l) = n.layout.as_mut() {
        if let Some(d) = values.get("direction") {
            match d.as_str() {
                "row" | "horizontal" | "row-reverse" => l.mode = LayoutMode::Horizontal,
                "column" | "vertical" | "column-reverse" => l.mode = LayoutMode::Vertical,
                "grid" => l.mode = LayoutMode::Grid,
                _ => {}
            }
        }
        if let Some(w) = values.get("wrap") {
            match w.as_str() {
                "true" | "wrap" | "1" | "yes" => l.wrap = true,
                "false" | "nowrap" | "no-wrap" | "0" | "no" => l.wrap = false,
                _ => {}
            }
        }
        if let Some(a) = values.get("mainAlign") {
            match a.as_str() {
                "start" | "flex-start" | "left" | "top" | "min" => l.main_align = MainAlign::Min,
                "center" => l.main_align = MainAlign::Center,
                "end" | "flex-end" | "right" | "bottom" | "max" => l.main_align = MainAlign::Max,
                "space-between" => l.main_align = MainAlign::SpaceBetween,
                "space-around" => l.main_align = MainAlign::SpaceAround,
                "space-evenly" => l.main_align = MainAlign::SpaceEvenly,
                _ => {}
            }
        }
        if let Some(a) = values.get("crossAlign") {
            match a.as_str() {
                "start" | "flex-start" | "left" | "top" | "min" => l.cross_align = CrossAlign::Min,
                "center" => l.cross_align = CrossAlign::Center,
                "end" | "flex-end" | "right" | "bottom" | "max" => l.cross_align = CrossAlign::Max,
                "baseline" => l.cross_align = CrossAlign::Baseline,
                _ => {}
            }
        }
    }
    if let Some(s) = values.get("sizingH").and_then(|s| sizing_of(s)) {
        n.child_layout.sizing_h = s;
    }
    if let Some(s) = values.get("sizingV").and_then(|s| sizing_of(s)) {
        n.child_layout.sizing_v = s;
    }
    if (n.layout.clone(), n.child_layout.clone()) != before {
        doc.mark_dirty([id]);
        if let Some(p) = doc.node(id).and_then(|n| n.parent) {
            doc.mark_dirty([p]);
        }
    }
}

// ---- commands -----------------------------------------------------------------------------------

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "layout.relative",
            "Relative size",
            "{ids?, axis: width|height, value? (number), unit? (%|vw|vh; none: the side stops being relative)} — `%` of the parent (its inner box when it is auto layout), vw / vh of the frame the layer sits in; kept in step after every edit",
            true,
            relative
        ),
        spec!(
            "layout.bindVariable",
            "Layout from a variable",
            "{ids?, field: direction|wrap|mainAlign|crossAlign|sizingH|sizingV, variable? (name or id of a string or boolean variable; none: unbind)} — the layout follows the variable in the layer's modes",
            true,
            bind
        ),
    ]
}

fn edit(doc: &mut Document, id: NodeId, f: impl FnOnce(&str) -> String) -> Result<()> {
    let n = doc.get_mut(id)?;
    let text = n.plugin_data.get(KEY).cloned().unwrap_or_default();
    let new = f(&text);
    if new.trim().is_empty() {
        n.plugin_data.remove(KEY);
    } else {
        n.plugin_data.insert(KEY.into(), new);
    }
    doc.mark_dirty([id]);
    Ok(())
}

fn relative(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layout.relative";
    let ids = crate::params::req_targets(cmd, s, p)?;
    let axis = req_string(cmd, p, "axis")?;
    let horizontal = match axis.as_str() {
        "width" | "w" => true,
        "height" | "h" => false,
        _ => return Err(bad(cmd, "axis: width|height")),
    };
    let unit = match string(p, "unit") {
        None => None,
        Some(u) => Some(Unit::parse(&u).ok_or_else(|| bad(cmd, "unit: %|vw|vh"))?),
    };
    let doc = s.doc_mut()?;
    for id in ids {
        let rule = match unit {
            Some(u) => {
                let value = match get(p, "value").and_then(Value::as_f64) {
                    Some(v) if v.is_finite() => v,
                    _ => {
                        // The share it has now.
                        let n = doc.get(id)?;
                        let now = if horizontal { n.width } else { n.height };
                        let base = px(doc, id, horizontal, 100.0, u).ok_or_else(|| bad(cmd, "nothing to be a share of: the layer needs a parent frame"))?;
                        if base > 1e-9 { now / base * 100.0 } else { 100.0 }
                    }
                };
                if reference(doc, id, u).is_none() {
                    return Err(bad(cmd, "nothing to be a share of: the layer needs a parent frame"));
                }
                Some(Rule::Size { horizontal, value, unit: u })
            }
            None => None,
        };
        edit(doc, id, |t| with_rule(t, |r| matches!(r, Rule::Size { horizontal: h, .. } if *h == horizontal), rule.as_ref()))?;
    }
    ok()
}

fn bind(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layout.bindVariable";
    let ids = crate::params::req_targets(cmd, s, p)?;
    let field = req_string(cmd, p, "field")?;
    if !VAR_FIELDS.contains(&field.as_str()) {
        return Err(bad(cmd, format!("field: {}", VAR_FIELDS.join("|"))));
    }
    let doc = s.doc_mut()?;
    let var = match string(p, "variable") {
        Some(v) => {
            let x = doc.variables.get(&v).ok_or_else(|| bad(cmd, "no such variable"))?;
            if !matches!(x.kind, figcraft_doc::VariableType::String | figcraft_doc::VariableType::Boolean) {
                return Err(bad(cmd, "a string or boolean variable"));
            }
            Some(x.name.clone())
        }
        None => None,
    };
    for id in ids {
        let rule = var.as_ref().map(|v| Rule::Var { field: field.clone(), variable: v.clone() });
        edit(doc, id, |t| with_rule(t, |r| matches!(r, Rule::Var { field: f, .. } if *f == field), rule.as_ref()))?;
    }
    ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(v: Value) -> NodeId {
        NodeId(v["id"].as_u64().unwrap())
    }

    fn size(s: &Session, n: NodeId) -> (f64, f64) {
        let n = s.doc().unwrap().node(n).unwrap();
        (n.width, n.height)
    }

    #[test]
    fn lines_round_trip() {
        for l in ["width 50%", "height 100vh", "width 33.3333vw", "direction {Layout/Card direction}"] {
            assert_eq!(format(&parse_line(l).unwrap()), l);
        }
        assert_eq!(parse_line("width 50"), None);
        assert_eq!(parse_line("colour {x}"), None);
    }

    #[test]
    fn a_share_of_the_parent_follows_it_until_resized_by_hand() {
        let mut s = Session::new();
        let f = id(s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 400, "height": 300})).unwrap());
        let r = id(s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 50, "height": 50, "parent": f})).unwrap());
        s.execute("layout.relative", &json!({"ids": [r], "axis": "width", "value": 50, "unit": "%"})).unwrap();
        assert_eq!(size(&s, r), (200.0, 50.0));
        s.execute("node.set", &json!({"ids": [f], "props": {"width": 800}})).unwrap();
        assert_eq!(size(&s, r).0, 400.0, "follows the parent");
        // vh of the frame it sits in.
        s.execute("layout.relative", &json!({"ids": [r], "axis": "height", "value": 50, "unit": "vh"})).unwrap();
        assert_eq!(size(&s, r).1, 150.0);
        // By hand: the width rule ends, the height one stays.
        s.execute("node.set", &json!({"ids": [r], "props": {"width": 120}})).unwrap();
        assert_eq!(size_rule(s.doc().unwrap(), r, true), None);
        assert!(size_rule(s.doc().unwrap(), r, false).is_some());
        s.execute("node.set", &json!({"ids": [f], "props": {"width": 600, "height": 400}})).unwrap();
        assert_eq!(size(&s, r), (120.0, 200.0));
        // Undo takes the rule and the size back together.
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(size(&s, r), (120.0, 150.0));
    }

    #[test]
    fn auto_layout_follows_string_variables_per_mode() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Layout"})).unwrap()["id"].as_str().unwrap().to_string();
        let v = s.execute("variable.create", &json!({"collection": c, "name": "Card/direction", "type": "string", "value": "row"})).unwrap()["id"].as_str().unwrap().to_string();
        let mobile = s.execute("variable.addMode", &json!({"collection": c, "name": "Mobile"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.set", &json!({"variable": v, "mode": mobile, "value": "column"})).unwrap();
        let f = id(s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 200})).unwrap());
        s.execute("node.set", &json!({"ids": [f], "props": {"layout": "vertical"}})).unwrap();
        s.execute("layout.bindVariable", &json!({"ids": [f], "field": "direction", "variable": "Card/direction"})).unwrap();
        let mode = |s: &Session| s.doc().unwrap().node(f).unwrap().layout.as_ref().unwrap().mode;
        assert_eq!(mode(&s), LayoutMode::Horizontal, "the default mode says row");
        s.execute("variable.setMode", &json!({"ids": [f], "collection": c, "mode": mobile})).unwrap();
        assert_eq!(mode(&s), LayoutMode::Vertical, "Mobile says column");
    }
}
