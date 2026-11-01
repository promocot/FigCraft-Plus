//! FigCraft Plus: the Design System Inspector — `designSystem.inspect`.
//!
//! One pass over the file that finds what keeps a design system from holding together:
//!
//! - **Not on a variable:** a solid fill or stroke colour, a corner radius, gap, padding, stroke
//!   weight or font size typed by hand although a variable (scoped for that property) holds exactly
//!   that value. The fix binds it.
//! - **Broken references:** an alias to a variable that's gone; a formula (derived colour,
//!   calculated number, a colour linked to number variables) reading a variable that's gone.
//! - **Circular formulas:** aliases or formulas that end up reading themselves.
//! - **Unused variables:** nothing binds, aliases or reads them. The fix deletes them.
//! - **Variables without a scope:** offered for every property (FigCraft's default).
//! - **Duplicate tokens:** variables of one collection holding the same values in every mode.
//! - **Invalid values:** a number that isn't finite; an alias to a variable of another type.
//!
//! Each finding names its layer or variable (to jump to it) and, where there's one, the command
//! that fixes it.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_color::{Color, PaintKind};
use figcraft_doc::{Document, Node, NodeId, NodeKind, Variable, VariableType, VariableValue};
use serde::Serialize;
use serde_json::{Value, json};

use super::CommandSpec;
use crate::oklch_links as links;
use crate::spec;
#[cfg(test)]
use crate::Session;

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "designSystem.inspect",
        "Inspect the design system",
        "{} → {findings: [{kind: unboundColor|unboundSize|brokenAlias|brokenFormula|circular|unused|noScope|duplicate|invalid, severity: error|warning|info, text, node?, variable?, collection?, fix? {command, params}}]}",
        false,
        |s, _| {
            let doc = s.doc()?;
            Ok(json!({"findings": inspect(doc)}))
        }
    )]
}

/// One thing the inspector found.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub kind: &'static str,
    pub severity: &'static str,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Value>,
}

impl Finding {
    fn new(kind: &'static str, severity: &'static str, text: String) -> Self {
        Finding { kind, severity, text, node: None, variable: None, collection: None, fix: None }
    }

    fn var(mut self, v: &Variable) -> Self {
        self.variable = Some(v.id.clone());
        self.collection = Some(v.collection.clone());
        self
    }
}

/// A variable by name or id.
fn find<'a>(doc: &'a Document, key: &str) -> Option<&'a Variable> {
    let vars = &doc.variables.variables;
    vars.iter().find(|v| v.id == key).or_else(|| vars.iter().find(|v| v.name == key))
}

/// A variable's value in its collection's first mode, aliases and calculations resolved.
fn default_value(doc: &Document, v: &Variable) -> Option<VariableValue> {
    let mode = doc.variables.collections.iter().find(|c| c.id == v.collection)?.modes.first()?.id.clone();
    let raw = v.values.get(&mode)?;
    doc.variables.flattener().value(v, &mode, raw)
}

fn same_color(a: Color, b: Color) -> bool {
    a.to_rgba8() == b.to_rgba8()
}

fn inside_instance(doc: &Document, id: NodeId) -> bool {
    doc.ancestors(id).iter().any(|a| doc.node(*a).is_some_and(|n| matches!(n.kind, NodeKind::Instance { .. })))
}

/// Everything the inspector finds in `doc`.
pub fn inspect(doc: &Document) -> Vec<Finding> {
    let mut out = Vec::new();
    let vars = &doc.variables.variables;
    let defaults: Vec<(&Variable, VariableValue)> = vars.iter().filter_map(|v| default_value(doc, v).map(|x| (v, x))).collect();
    // ---- values typed by hand that a variable holds ----
    for n in doc.nodes() {
        if n.is_page() || inside_instance(doc, n.id) {
            continue;
        }
        unbound_colors(doc, n, &defaults, &mut out);
        unbound_sizes(n, &defaults, &mut out);
    }
    // ---- references ----
    let mut used: BTreeSet<String> = BTreeSet::new();
    for n in doc.nodes() {
        used.extend(n.bound.values().cloned());
        if !n.interactions.is_empty()
            && let Ok(text) = serde_json::to_string(&n.interactions)
        {
            used.extend(vars.iter().filter(|v| text.contains(&v.id)).map(|v| v.id.clone()));
        }
        if let Some(text) = n.plugin_data.get(links::NODE_KEY) {
            for (_, f) in links::formulas_in(text) {
                for r in f.reads() {
                    match find(doc, &r) {
                        Some(v) => {
                            used.insert(v.id.clone());
                        }
                        None => {
                            let mut f = Finding::new("brokenFormula", "error", format!("“{}” uses a formula reading “{r}”, which doesn't exist", n.name));
                            f.node = Some(n.id);
                            out.push(f);
                        }
                    }
                }
            }
        }
    }
    for s in &doc.styles {
        used.extend(s.bound.values().cloned());
        for (_, f) in links::formulas_in(&s.description) {
            used.extend(f.reads().iter().filter_map(|r| find(doc, r)).map(|v| v.id.clone()));
        }
    }
    // Variables reading others: aliases, calculations, formulas.
    let mut reads: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for v in vars {
        let mut r: Vec<String> = Vec::new();
        for val in v.values.values() {
            match val {
                VariableValue::Alias(t) => match doc.variables.get_by_id(t) {
                    Some(target) => {
                        if target.kind != v.kind {
                            out.push(
                                Finding::new("invalid", "error", format!("“{}” aliases “{}”, a {} variable", v.name, target.name, target.kind.id()))
                                    .var(v),
                            );
                        }
                        r.push(target.id.clone());
                    }
                    None => out.push(Finding::new("brokenAlias", "error", format!("“{}” aliases a variable that doesn't exist", v.name)).var(v)),
                },
                VariableValue::ColorAlpha { variable, .. } => r.push(variable.clone()),
                VariableValue::ColorMix { from, to, .. } => {
                    for o in [from, to] {
                        if let figcraft_doc::ColorOperand::Variable { variable } = o {
                            r.push(variable.clone());
                        }
                    }
                }
                VariableValue::Number(x) if !x.is_finite() => out.push(Finding::new("invalid", "error", format!("“{}” isn't a finite number", v.name)).var(v)),
                _ => {}
            }
        }
        let formula_reads: Vec<String> = match v.kind {
            VariableType::Color => links::formula_in(&v.description, None).map(|f| f.reads()).unwrap_or_default(),
            VariableType::Number => links::number_formula_in(&v.description)
                .map(|e| {
                    let mut out = Vec::new();
                    links::Formula { model: links::Model::Rgb, ch: [links::Channel::Expr(e), links::Channel::Fixed(0.0), links::Channel::Fixed(0.0), links::Channel::Fixed(100.0)], derive: None }
                        .reads()
                        .into_iter()
                        .for_each(|x| out.push(x));
                    out
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        for name in formula_reads {
            match find(doc, &name) {
                Some(t) => r.push(t.id.clone()),
                None => out.push(Finding::new("brokenFormula", "error", format!("“{}”'s formula reads “{name}”, which doesn't exist", v.name)).var(v)),
            }
        }
        used.extend(r.iter().cloned());
        reads.insert(v.id.clone(), r);
    }
    // Extended collections inherit.
    used.extend(vars.iter().filter_map(|v| v.extends.clone()));
    // ---- cycles ----
    let mut reported: BTreeSet<String> = BTreeSet::new();
    for v in vars {
        if reported.contains(&v.id) {
            continue;
        }
        if let Some(cycle) = cycle_from(&v.id, &reads) {
            let names: Vec<String> = cycle.iter().filter_map(|id| doc.variables.get_by_id(id)).map(|x| x.name.clone()).collect();
            reported.extend(cycle.iter().cloned());
            out.push(Finding::new("circular", "error", format!("{} read each other in a circle", names.join(" → "))).var(v));
        }
    }
    // ---- unused, scopes, duplicates ----
    for v in vars.iter().filter(|v| !figcraft_engine_is_remote(doc, &v.collection)) {
        if !used.contains(&v.id) {
            let mut f = Finding::new("unused", "warning", format!("“{}” isn't used", v.name)).var(v);
            f.fix = Some(json!({"command": "variable.delete", "params": {"variable": v.id}}));
            out.push(f);
        }
        if matches!(v.kind, VariableType::Color | VariableType::Number)
            && v.scopes.as_ref().is_none_or(|s| s.iter().any(|x| x == figcraft_doc::ALL_SCOPES))
        {
            out.push(Finding::new("noScope", "info", format!("“{}” is offered for every property: give it a scope", v.name)).var(v));
        }
    }
    let mut groups: BTreeMap<(String, String), Vec<&Variable>> = BTreeMap::new();
    for v in vars.iter().filter(|v| !v.values.values().any(|x| matches!(x, VariableValue::Alias(_)))) {
        if let Ok(key) = serde_json::to_string(&v.values) {
            // Colours anywhere in a collection; numbers and text in one group (a radius of 16 and a
            // space of 16 aren't the same token).
            let group = if v.kind == VariableType::Color { "" } else { v.name.rsplit_once('/').map_or("", |(g, _)| g) };
            groups.entry((v.collection.clone(), format!("{}:{group}:{key}", v.kind.id()))).or_default().push(v);
        }
    }
    for list in groups.values().filter(|l| l.len() > 1) {
        let names: Vec<String> = list.iter().map(|v| format!("“{}”", v.name)).collect();
        out.push(Finding::new("duplicate", "info", format!("{} hold the same values", names.join(", "))).var(list[0]));
    }
    out
}

fn figcraft_engine_is_remote(doc: &Document, collection: &str) -> bool {
    crate::library::is_remote_collection(doc, collection)
}

/// A circle through `start` in `reads` (its ids, in order), if any.
fn cycle_from(start: &str, reads: &BTreeMap<String, Vec<String>>) -> Option<Vec<String>> {
    fn walk(at: &str, start: &str, reads: &BTreeMap<String, Vec<String>>, path: &mut Vec<String>, seen: &mut BTreeSet<String>) -> bool {
        for next in reads.get(at).into_iter().flatten() {
            if next == start {
                return true;
            }
            if seen.insert(next.clone()) {
                path.push(next.clone());
                if walk(next, start, reads, path, seen) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut path = vec![start.to_string()];
    let mut seen = BTreeSet::new();
    walk(start, start, reads, &mut path, &mut seen).then_some(path)
}

/// The scope a fill of `n` needs.
fn fill_scope(n: &Node) -> &'static str {
    match n.kind {
        NodeKind::Text(_) => "TEXT_FILL",
        _ if n.kind.is_frame_like() => "FRAME_FILL",
        _ => "SHAPE_FILL",
    }
}

fn unbound_colors(doc: &Document, n: &Node, defaults: &[(&Variable, VariableValue)], out: &mut Vec<Finding>) {
    let lists = [(true, &n.fills, n.styles.fill.is_some()), (false, &n.strokes, n.styles.stroke.is_some())];
    for (fills, paints, styled) in lists {
        if styled {
            continue;
        }
        for (i, p) in paints.iter().enumerate() {
            let PaintKind::Solid { color } = &p.kind else { continue };
            if !p.visible {
                continue;
            }
            let field = format!("{}/{i}/color", if fills { "fills" } else { "strokes" });
            let short = if fills { "fill" } else { "stroke" };
            let bound = n.bound.contains_key(&field) || (i == 0 && (n.bound.contains_key(short) || n.bound.contains_key(&format!("{}/0", if fills { "fills" } else { "strokes" }))));
            if bound {
                continue;
            }
            let scope = if fills { fill_scope(n) } else { "STROKE_COLOR" };
            let found: Vec<&&Variable> = defaults
                .iter()
                .filter(|(v, x)| v.kind == VariableType::Color && v.in_scope(scope) && matches!(x, VariableValue::Color(c) if same_color(*c, *color)))
                .map(|(v, _)| v)
                .collect();
            let Some(v) = found.first() else { continue };
            let mut f = Finding::new(
                "unboundColor",
                "warning",
                format!("“{}”: {} {} is “{}”'s value, typed by hand", n.name, short, color.to_css_hex(), v.name),
            );
            f.node = Some(n.id);
            f.variable = Some(v.id.clone());
            f.fix = Some(json!({"command": "variable.bind", "params": {"ids": [n.id], "field": field, "variable": v.id}}));
            out.push(f);
            let _ = doc;
        }
    }
}

fn unbound_sizes(n: &Node, defaults: &[(&Variable, VariableValue)], out: &mut Vec<Finding>) {
    let mut sizes: Vec<(&str, &str, f64)> = Vec::new();
    if n.kind.has_corner_radius()
        && let Some(r) = n.corners.uniform()
    {
        sizes.push(("cornerRadius", "CORNER_RADIUS", r));
    }
    if let Some(l) = &n.layout {
        sizes.push(("gap", "GAP", l.gap));
        let [t, r, b, lft] = l.padding;
        for (field, v) in [("paddingTop", t), ("paddingRight", r), ("paddingBottom", b), ("paddingLeft", lft)] {
            sizes.push((field, "GAP", v));
        }
    }
    if !n.strokes.is_empty() && n.stroke.sides.is_none() {
        sizes.push(("strokeWeight", "STROKE_FLOAT", n.stroke.weight));
    }
    if let NodeKind::Text(t) = &n.kind
        && n.styles.text.is_none()
        && t.runs.is_empty()
    {
        sizes.push(("fontSize", "FONT_SIZE", t.style.font_size));
    }
    for (field, scope, x) in sizes {
        if x <= 0.0 || !x.is_finite() || n.bound.contains_key(field) {
            continue;
        }
        // Padding bound as a whole or per axis.
        if field.starts_with("padding") && ["padding", "paddingHorizontal", "paddingVertical"].iter().any(|k| n.bound.contains_key(*k)) {
            continue;
        }
        let fits: Vec<&Variable> = defaults
            .iter()
            .filter(|(v, val)| v.kind == VariableType::Number && v.in_scope(scope) && matches!(val, VariableValue::Number(y) if (y - x).abs() < 1e-6))
            .map(|(v, _)| *v)
            .collect();
        // A variable scoped for it; else one offered everywhere whose name says what it is for.
        let hints: &[&str] = match scope {
            "CORNER_RADIUS" => &["radius", "radii", "corner", "rounded"],
            "GAP" => &["space", "spacing", "gap", "padding", "inset"],
            "STROKE_FLOAT" => &["stroke", "border"],
            _ => &["font", "text", "size", "type"],
        };
        let named = |v: &&&Variable| {
            let n = v.name.to_lowercase();
            hints.iter().any(|h| n.contains(h))
        };
        let Some(v) = fits.iter().find(|v| v.scopes.is_some() && !v.scopes.as_ref().is_some_and(|s| s.iter().any(|x| x == figcraft_doc::ALL_SCOPES))).or_else(|| fits.iter().find(named)) else {
            continue;
        };
        let mut f = Finding::new("unboundSize", "warning", format!("“{}”: {field} {} is “{}”'s value, typed by hand", n.name, links_num(x), v.name));
        f.node = Some(n.id);
        f.variable = Some(v.id.clone());
        f.fix = Some(json!({"command": "variable.bind", "params": {"ids": [n.id], "field": field, "variable": v.id}}));
        out.push(f);
    }
}

fn links_num(x: f64) -> String {
    figcraft_doc::trim_num(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(s: &Session) -> Vec<&'static str> {
        inspect(s.doc().unwrap()).iter().map(|f| f.kind).collect()
    }

    #[test]
    fn finds_unbound_values_broken_and_unused_variables_and_fixes_them() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Tokens"})).unwrap()["id"].as_str().unwrap().to_string();
        let brand = s.execute("variable.create", &json!({"collection": c, "name": "brand", "type": "color", "value": "#3366FF"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": brand, "scopes": ["ALL_FILLS"]})).unwrap();
        let radius = s.execute("variable.create", &json!({"collection": c, "name": "radius/m", "type": "number", "value": 8})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": radius, "scopes": ["CORNER_RADIUS"]})).unwrap();
        let unused = s.execute("variable.create", &json!({"collection": c, "name": "old", "type": "number", "value": 3})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": unused, "scopes": ["GAP"]})).unwrap();
        let dup = s.execute("variable.create", &json!({"collection": c, "name": "brand-copy", "type": "color", "value": "#3366FF"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": dup, "scopes": ["STROKE_COLOR"]})).unwrap();
        let calc = s.execute("variable.create", &json!({"collection": c, "name": "calc", "type": "number", "value": 1})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": calc, "scopes": ["GAP"], "description": "calc({gone} * 2)"})).unwrap();
        let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 50, "height": 50, "props": {"fill": "#3366FF", "cornerRadius": 8}})).unwrap()["id"].as_u64().unwrap();
        let k = kinds(&s);
        for want in ["unboundColor", "unboundSize", "unused", "duplicate", "brokenFormula"] {
            assert!(k.contains(&want), "{want} in {k:?}");
        }
        assert!(!k.contains(&"noScope"), "every variable here has a scope");
        // Fix the layer: both values bound; `brand` and `radius/m` are used now.
        for f in inspect(s.doc().unwrap()).into_iter().filter(|f| f.node == Some(NodeId(r))) {
            let fix = f.fix.unwrap();
            s.execute(fix["command"].as_str().unwrap(), &fix["params"]).unwrap();
        }
        let after = inspect(s.doc().unwrap());
        assert!(!after.iter().any(|f| f.kind.starts_with("unbound")), "{after:?}");
        assert!(after.iter().any(|f| f.kind == "unused" && f.variable.as_deref() == Some(unused.as_str())));
        assert!(!after.iter().any(|f| f.kind == "unused" && f.variable.as_deref() == Some(brand.as_str())));
    }

    #[test]
    fn circular_aliases_are_found() {
        let mut reads = BTreeMap::new();
        reads.insert("a".to_string(), vec!["b".to_string()]);
        reads.insert("b".to_string(), vec!["c".to_string()]);
        reads.insert("c".to_string(), vec!["a".to_string()]);
        reads.insert("d".to_string(), vec!["a".to_string()]);
        assert_eq!(cycle_from("a", &reads), Some(vec!["a".into(), "b".into(), "c".into()]));
        assert_eq!(cycle_from("d", &reads), None, "d reads the circle but isn't in it");
    }
}
