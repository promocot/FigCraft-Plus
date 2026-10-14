//! OKLCH links: a colour variable whose description holds a line `oklch(L C H)`, where any channel
//! may name a number variable in braces (`oklch(62% 0.19 {Hue})`), keeps its value computed from
//! them in every mode of its collection. Change `Hue` from 280 to 130 and every linked colour (and
//! every layer bound to one) follows.
//!
//! Channels: L is a percentage (`62%`, or a number variable holding 0–100), C is chroma (`0.19`, or
//! `48%` of 0.4 as in CSS, or a number variable holding 0–0.4), H is degrees. A number variable in
//! the colour's own collection is read in the same mode (Light/Dark lightness), any other in its
//! default mode. Editing the colour itself (hex, picker) rewrites the formula's fixed channels
//! from the new colour; linked channels stay linked. The formula is plain text, so a FigCraft
//! without this feature keeps the last computed colours and the description as it is.

use std::collections::BTreeMap;

use figcraft_color::{Color, OKLCH_FULL_CHROMA, Oklch};
use figcraft_doc::{Document, VariableType, VariableValue};

/// One channel of a formula: a fixed value or a number variable's name.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Fixed(f64),
    Var(String),
}

/// `oklch(L C H)` with L 0..1, C 0..0.5 and H degrees when fixed.
#[derive(Clone, Debug, PartialEq)]
pub struct Formula {
    pub l: Channel,
    pub c: Channel,
    pub h: Channel,
}

/// The formula line in a variable description, if any.
pub fn parse(description: &str) -> Option<Formula> {
    description.lines().find_map(parse_line)
}

fn parse_line(line: &str) -> Option<Formula> {
    let t = line.trim();
    let inner = t.get(..6).filter(|p| p.eq_ignore_ascii_case("oklch("))?;
    let rest = t.get(inner.len()..)?.strip_suffix(')')?;
    let parts = split_channels(rest)?;
    let [l, c, h] = parts.as_slice() else { return None };
    Some(Formula { l: channel(l, |v, pct| if pct { v / 100.0 } else if v > 1.0 { v / 100.0 } else { v })?, c: channel(c, |v, pct| if pct { v / 100.0 * OKLCH_FULL_CHROMA } else { v })?, h: channel(h, |v, _| v)? })
}

/// Split on whitespace outside braces (variable names may hold spaces).
fn split_channels(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '{' => {
                depth += 1;
                cur.push(ch);
            }
            '}' => {
                depth = depth.checked_sub(1)?;
                cur.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if depth != 0 {
        return None;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Some(out)
}

fn channel(token: &str, fixed: impl Fn(f64, bool) -> f64) -> Option<Channel> {
    if let Some(name) = token.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        let name = name.trim();
        return (!name.is_empty()).then(|| Channel::Var(name.to_string()));
    }
    let token = token.trim_end_matches("deg");
    let (num, pct) = match token.strip_suffix('%') {
        Some(n) => (n, true),
        None => (token, false),
    };
    num.parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| Channel::Fixed(fixed(v, pct)))
}

fn trim(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// The formula as its description line.
pub fn format(f: &Formula) -> String {
    let ch = |c: &Channel, fixed: &dyn Fn(f64) -> String| match c {
        Channel::Fixed(v) => fixed(*v),
        Channel::Var(n) => format!("{{{n}}}"),
    };
    format!(
        "oklch({} {} {})",
        ch(&f.l, &|v| format!("{}%", trim(v * 100.0, 2))),
        ch(&f.c, &|v| trim(v, 4)),
        ch(&f.h, &|v| trim(v, 2))
    )
}

/// `description` with its formula line replaced by `f` (added when missing; removed for None).
pub fn with_formula(description: &str, f: Option<&Formula>) -> String {
    let mut lines: Vec<String> = description.lines().filter(|l| parse_line(l).is_none()).map(str::to_string).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if let Some(f) = f {
        lines.push(format(f));
    }
    lines.join("\n")
}

/// A formula with every channel fixed to `c`'s values.
pub fn fixed_from(c: Oklch) -> Formula {
    Formula { l: Channel::Fixed(c.l), c: Channel::Fixed(c.c), h: Channel::Fixed(c.h) }
}

/// Keep the linked channels, take the fixed ones from `c`.
fn refix(f: &Formula, c: Oklch) -> Formula {
    let pick = |ch: &Channel, v: f64| match ch {
        Channel::Var(_) => ch.clone(),
        Channel::Fixed(_) => Channel::Fixed(v),
    };
    Formula { l: pick(&f.l, c.l), c: pick(&f.c, c.c), h: pick(&f.h, c.h) }
}

/// The value of number variable `name` in `modes` (None: missing or not a number).
fn number(doc: &Document, name: &str, modes: &BTreeMap<String, String>) -> Option<f64> {
    let v = doc.variables.get(name)?;
    if v.kind != VariableType::Number {
        return None;
    }
    match doc.variables.resolve(&v.id, modes)? {
        VariableValue::Number(n) if n.is_finite() => Some(n),
        _ => None,
    }
}

/// The OKLCH a formula gives in `modes` (None when a linked variable is missing).
pub fn evaluate(doc: &Document, f: &Formula, modes: &BTreeMap<String, String>) -> Option<Oklch> {
    let get = |ch: &Channel, scale: f64| match ch {
        Channel::Fixed(v) => Some(*v),
        Channel::Var(n) => number(doc, n, modes).map(|x| x * scale),
    };
    Some(Oklch::new(get(&f.l, 0.01)?, get(&f.c, 1.0)?, get(&f.h, 1.0)?))
}

/// After an edit that changed variables: recompute every linked colour variable (see the module
/// docs). Called by the engine's normalisation, inside the edit's undo step.
pub fn resolve(doc: &mut Document, before: &Document) {
    if doc.variables == before.variables {
        return;
    }
    let profile = doc.color_profile;
    let mut updates: Vec<(String, BTreeMap<String, VariableValue>, Option<String>)> = Vec::new();
    for v in doc.variables.variables.iter().filter(|v| v.kind == VariableType::Color) {
        let Some(mut f) = parse(&v.description) else { continue };
        let Some(coll) = doc.variables.collection(&v.collection) else { continue };
        let mut description = None;
        // The colour edited directly (same formula as before, a value changed): its fixed
        // channels follow the edit.
        if let Some(prev) = before.variables.variables.iter().find(|p| p.id == v.id)
            && parse(&prev.description).as_ref() == Some(&f)
            && let Some(VariableValue::Color(c)) = coll.modes.iter().find_map(|m| match (v.values.get(&m.id), prev.values.get(&m.id)) {
                (Some(now), Some(was)) if now != was => Some(now.clone()),
                _ => None,
            })
        {
            f = refix(&f, Oklch::from_color(c, profile));
            description = Some(with_formula(&v.description, Some(&f)));
        }
        let mut values = v.values.clone();
        for m in &coll.modes {
            // An alias in a mode is the user's choice: leave it.
            if matches!(values.get(&m.id), Some(VariableValue::Alias(_))) {
                continue;
            }
            let modes = BTreeMap::from([(coll.id.clone(), m.id.clone())]);
            let Some(lch) = evaluate(doc, &f, &modes) else { continue };
            let alpha = match values.get(&m.id) {
                Some(VariableValue::Color(c)) => c.a,
                _ => 1.0,
            };
            let col: Color = lch.to_color(profile, alpha);
            values.insert(m.id.clone(), VariableValue::Color(col));
        }
        if values != v.values || description.is_some() {
            updates.push((v.id.clone(), values, description));
        }
    }
    for (id, values, description) in updates {
        if let Some(v) = doc.variables.variables.iter_mut().find(|v| v.id == id) {
            v.values = values;
            if let Some(d) = description {
                v.description = d;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::Session;

    #[test]
    fn formulas_parse_and_format() {
        let f = parse("Brand primary\noklch(62% 0.19 {Brand/Hue})").unwrap();
        assert_eq!(f, Formula { l: Channel::Fixed(0.62), c: Channel::Fixed(0.19), h: Channel::Var("Brand/Hue".into()) });
        assert_eq!(format(&f), "oklch(62% 0.19 {Brand/Hue})");
        let g = parse("OKLCH({Light} 50% 120deg)").unwrap();
        assert_eq!(g, Formula { l: Channel::Var("Light".into()), c: Channel::Fixed(0.2), h: Channel::Fixed(120.0) });
        assert_eq!(parse("oklch({My hue} 0.1 2)").map(|f| f.l), Some(Channel::Var("My hue".into())));
        assert!(parse("oklch(1 2)").is_none() && parse("oklch({x 1 2)").is_none() && parse("rgb(1 2 3)").is_none());
        assert_eq!(with_formula("note\noklch(1% 0 0)\n", Some(&f)), "note\noklch(62% 0.19 {Brand/Hue})");
        assert_eq!(with_formula("note\noklch(1% 0 0)", None), "note");
        assert_eq!(with_formula("", Some(&f)), "oklch(62% 0.19 {Brand/Hue})");
    }

    fn color_of(s: &Session, name: &str, mode: &str) -> Oklch {
        let doc = s.doc().unwrap();
        let v = doc.variables.get(name).unwrap();
        let coll = doc.variables.collection(&v.collection).unwrap();
        let m = coll.find_mode(mode).unwrap();
        match v.values.get(&m.id) {
            Some(VariableValue::Color(c)) => Oklch::from_color(*c, doc.color_profile),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_hue_variable_drives_linked_colours_and_their_layers() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Theme", "modes": ["Light", "Dark"]})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.create", &json!({"collection": c, "name": "Hue", "type": "number", "value": 280})).unwrap();
        s.execute("variable.create", &json!({"collection": c, "name": "Lightness", "type": "number", "values": {"Light": 62, "Dark": 80}})).unwrap();
        let p = s.execute("variable.create", &json!({"collection": c, "name": "Primary", "type": "color", "value": "#000000"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": p, "description": "Main brand colour\noklch({Lightness} 0.15 {Hue})"})).unwrap();
        let light = color_of(&s, "Primary", "Light");
        assert!((light.l - 0.62).abs() < 0.01 && (light.h - 280.0).abs() < 1.0 && (light.c - 0.15).abs() < 0.01, "{light:?}");
        assert!((color_of(&s, "Primary", "Dark").l - 0.80).abs() < 0.01);
        // A layer bound to it follows a hue change, in the same undo step.
        let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap()["id"].as_u64().unwrap();
        s.execute("variable.bind", &json!({"ids": [r], "field": "fill", "variable": p})).unwrap();
        s.execute("variable.set", &json!({"variable": "Hue", "mode": "Light", "value": 130})).unwrap();
        assert!((color_of(&s, "Primary", "Light").h - 130.0).abs() < 1.0);
        let node = s.execute("node.get", &json!({"id": r})).unwrap();
        let fill = Color::from_hex(node["fills"][0]["color"].as_str().unwrap()).unwrap();
        assert!((Oklch::from_color(fill, figcraft_color::ColorProfile::Srgb).h - 130.0).abs() < 1.5, "{node}");
        s.execute("edit.undo", &json!({})).unwrap();
        assert!((color_of(&s, "Primary", "Light").h - 280.0).abs() < 1.0, "undo takes the colour back with the hue");
        // Editing the colour itself refixes the fixed channels and keeps the links.
        s.execute("variable.set", &json!({"variable": p, "mode": "Light", "value": "#FF0000"})).unwrap();
        let f = parse(&s.doc().unwrap().variables.get("Primary").unwrap().description).unwrap();
        assert_eq!((&f.l, &f.h), (&Channel::Var("Lightness".into()), &Channel::Var("Hue".into())));
        assert!(matches!(f.c, Channel::Fixed(c) if (c - 0.2577).abs() < 0.002), "{f:?}");
        assert!(s.doc().unwrap().variables.get("Primary").unwrap().description.starts_with("Main brand colour\n"));
        // A missing variable leaves the colour as it was.
        s.execute("variable.update", &json!({"variable": p, "description": "oklch(50% 0.1 {Nope})"})).unwrap();
        assert!((color_of(&s, "Primary", "Light").h - 280.0).abs() < 1.0);
    }
}
