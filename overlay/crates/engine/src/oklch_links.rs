//! Colours driven by number variables (FigCraft Plus).
//!
//! Any colour — a layer's fill or stroke, a gradient stop, an effect's colour, a page background,
//! a colour style's paint, a colour variable — can have a *formula*: its model (OKLCH, HSL, HSB or
//! RGB) and, per channel, either a fixed value or a number variable. The engine keeps the colour
//! equal to the formula after every edit, so changing a variable (`Hue` 280 → 130) restyles every
//! colour linked to it, in the same undo step.
//!
//! A formula is one line of text: `[slot ]model(c1 c2 c3[ / alpha])`, where a channel is a number
//! or `{Variable name}`:
//!
//! ```text
//! oklch(0.62 0.19 {Hue})                 a colour variable (its description)
//! paints/0 hsl({Hue} 70 {Light} / 100)   a colour style's first paint (its description)
//! fills/0 rgb(255 {G} 0)                 a layer's first fill (layer data `figcraftplus.colors`)
//! fills/1/stops/2 oklch(…)               a gradient stop
//! effects/0 oklch(…)  effects/0/secondary …   an effect's colour (a duo noise's second one)
//! background oklch(…)                     a page's background (on the page)
//! ```
//!
//! Units are the colour picker's: OKLCH L 0–1 (a variable above 1 is read as a percentage), C 0–0.4,
//! H degrees; HSL / HSB H degrees, S L B 0–100; RGB 0–255; alpha 0–100. Fixed values also take
//! `%` (OKLCH L `62%`, C `50%` of 0.4). A variable is found by name (or id) and read in the modes
//! that apply where the colour is (a layer's frames, a colour variable's own collection mode).
//! Editing the colour itself (hex, picker, eyedropper) rewrites the formula's fixed channels from
//! it; linked channels stay linked. Without FigCraft Plus the colours simply keep their last values.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_color::{Color, ColorProfile, Hsb, Hsl, OKLCH_FULL_CHROMA, Oklch, PaintKind};
use figcraft_doc::{Document, NodeId, NodeKind, StyleKind, VariableType, VariableValue};

/// The layer data key holding a layer's (or page's) colour formulas, one per line.
pub const NODE_KEY: &str = "figcraftplus.colors";

/// A colour model a formula is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Oklch,
    Hsl,
    Hsb,
    Rgb,
}

impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Model::Oklch => "oklch",
            Model::Hsl => "hsl",
            Model::Hsb => "hsb",
            Model::Rgb => "rgb",
        }
    }

    /// The picker's model menu label ("OKLCH", "HSL", …).
    pub fn label(self) -> &'static str {
        match self {
            Model::Oklch => "OKLCH",
            Model::Hsl => "HSL",
            Model::Hsb => "HSB",
            Model::Rgb => "RGB",
        }
    }

    pub fn from_name(s: &str) -> Option<Model> {
        match s.trim().to_ascii_lowercase().as_str() {
            "oklch" => Some(Model::Oklch),
            "hsl" => Some(Model::Hsl),
            "hsb" | "hsv" => Some(Model::Hsb),
            "rgb" => Some(Model::Rgb),
            _ => None,
        }
    }

    /// The channel letters, alpha last.
    pub fn letters(self) -> [&'static str; 4] {
        match self {
            Model::Oklch => ["L", "C", "H", "A"],
            Model::Hsl => ["H", "S", "L", "A"],
            Model::Hsb => ["H", "S", "B", "A"],
            Model::Rgb => ["R", "G", "B", "A"],
        }
    }

    /// `c`'s channel values in this model, in the picker's units (alpha 0–100 last).
    pub fn values(self, c: Color, profile: ColorProfile) -> [f64; 4] {
        let a = f64::from(c.a) * 100.0;
        match self {
            Model::Oklch => {
                let o = Oklch::from_color(c, profile);
                [o.l, o.c, o.h, a]
            }
            Model::Hsl => {
                let x = Hsl::from_color(c);
                [f64::from(x.h), f64::from(x.s) * 100.0, f64::from(x.l) * 100.0, a]
            }
            Model::Hsb => {
                let x = Hsb::from_color(c);
                [f64::from(x.h), f64::from(x.s) * 100.0, f64::from(x.b) * 100.0, a]
            }
            Model::Rgb => [f64::from(c.r) * 255.0, f64::from(c.g) * 255.0, f64::from(c.b) * 255.0, a],
        }
    }

    /// The colour of channel values `v` (picker units), in `profile`, rounded to what its hex shows.
    pub fn color(self, v: [f64; 4], profile: ColorProfile) -> Color {
        let fin = |x: f64| if x.is_finite() { x } else { 0.0 };
        let a = (fin(v[3]) / 100.0).clamp(0.0, 1.0) as f32;
        let pct = |x: f64| (fin(x) / 100.0).clamp(0.0, 1.0) as f32;
        let hue = |x: f64| fin(x).rem_euclid(360.0) as f32;
        let c = match self {
            Model::Oklch => Oklch::new(v[0], v[1], v[2]).to_color(profile, a),
            Model::Hsl => Hsl { h: hue(v[0]), s: pct(v[1]), l: pct(v[2]) }.to_color().with_alpha(a),
            Model::Hsb => Hsb { h: hue(v[0]), s: pct(v[1]), b: pct(v[2]) }.to_color().with_alpha(a),
            Model::Rgb => {
                let ch = |x: f64| (fin(x) / 255.0).clamp(0.0, 1.0) as f32;
                Color::new(ch(v[0]), ch(v[1]), ch(v[2]), a)
            }
        };
        snap(c)
    }
}

/// `c` rounded to 8 bits a channel: what its hex shows, so the values shown for it are exact.
pub fn snap(c: Color) -> Color {
    let [r, g, b, a] = c.to_rgba8();
    Color::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, f32::from(a) / 255.0)
}

/// One channel: a fixed value (picker units) or a number variable's name.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Fixed(f64),
    Var(String),
}

/// A colour formula: its model, three channels and alpha.
#[derive(Clone, Debug, PartialEq)]
pub struct Formula {
    pub model: Model,
    /// The model's three channels, then alpha.
    pub ch: [Channel; 4],
}

impl Formula {
    /// Every channel fixed to `c`'s values.
    pub fn fixed(model: Model, c: Color, profile: ColorProfile) -> Formula {
        let v = model.values(c, profile);
        Formula { model, ch: v.map(Channel::Fixed) }
    }

    /// Whether any channel is linked to a variable.
    pub fn linked(&self) -> bool {
        self.ch.iter().any(|c| matches!(c, Channel::Var(_)))
    }

    /// The variable channel `k` is linked to.
    pub fn var(&self, k: usize) -> Option<&str> {
        match self.ch.get(k)? {
            Channel::Var(n) => Some(n),
            Channel::Fixed(_) => None,
        }
    }

    /// The fixed channels taken from `c`; linked ones kept.
    pub fn refixed(&self, c: Color, profile: ColorProfile) -> Formula {
        let v = self.model.values(c, profile);
        let mut ch = self.ch.clone();
        for (k, slot) in ch.iter_mut().enumerate() {
            if let Channel::Fixed(_) = slot {
                *slot = Channel::Fixed(v[k.min(3)]);
            }
        }
        Formula { model: self.model, ch }
    }

    /// The colour in `modes` (None when a linked variable is missing or not a number).
    pub fn evaluate(&self, doc: &Document, modes: &BTreeMap<String, String>) -> Option<Color> {
        let mut v = [0.0; 4];
        for (k, c) in self.ch.iter().enumerate() {
            let x = match c {
                Channel::Fixed(x) => *x,
                Channel::Var(n) => {
                    let x = number(doc, n, modes)?;
                    // OKLCH lightness from a variable: 0–1, or a percentage above 1.
                    if self.model == Model::Oklch && k == 0 && x > 1.0 { x / 100.0 } else { x }
                }
            };
            if let Some(slot) = v.get_mut(k) {
                *slot = x;
            }
        }
        Some(self.model.color(v, doc.color_profile))
    }
}

/// The value of number variable `name` (or id) in `modes`.
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

// ---- text ---------------------------------------------------------------------------------------

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
            c if (c.is_whitespace() || c == ',') && depth == 0 => {
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

/// A channel token; `pct` scales a `%` value (what 100% means), `fix` adjusts a bare number.
fn channel(token: &str, pct: f64, fix: impl Fn(f64) -> f64) -> Option<Channel> {
    let t = token.trim();
    if let Some(name) = t.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        let name = name.trim();
        return (!name.is_empty()).then(|| Channel::Var(name.to_string()));
    }
    let t = t.trim_end_matches("deg");
    let (num, is_pct) = match t.strip_suffix('%') {
        Some(n) => (n, true),
        None => (t, false),
    };
    let v = num.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
    Some(Channel::Fixed(if is_pct { v / 100.0 * pct } else { fix(v) }))
}

/// Parse a formula line: `(slot, formula)`.
pub fn parse_line(line: &str) -> Option<(Option<String>, Formula)> {
    let t = line.trim();
    let open = t.find('(')?;
    let head = t.get(..open)?.trim();
    let (slot, model) = match head.rsplit_once(char::is_whitespace) {
        Some((s, m)) => (Some(s.trim().to_string()), m),
        None => (None, head),
    };
    let model = Model::from_name(model)?;
    if slot.as_deref().is_some_and(|s| s.is_empty() || s.contains(char::is_whitespace)) {
        return None;
    }
    let inner = t.get(open + 1..)?.strip_suffix(')')?;
    let (main, alpha) = match inner.split_once('/') {
        // A '/' inside braces is part of a variable name ("Brand/Hue").
        Some((m, a)) if m.matches('{').count() == m.matches('}').count() => (m, Some(a)),
        _ => match inner.rfind(" / ") {
            Some(i) => (inner.get(..i)?, inner.get(i + 3..)),
            None => (inner, None),
        },
    };
    let parts = split_channels(main)?;
    let [a, b, c] = parts.as_slice() else { return None };
    let (a, b, c) = match model {
        // L: 0–1, `62%`, or a bare 62 meaning percent. C: `50%` of 0.4.
        Model::Oklch => (channel(a, 1.0, |v| if v > 1.0 { v / 100.0 } else { v })?, channel(b, OKLCH_FULL_CHROMA, |v| v)?, channel(c, 360.0, |v| v)?),
        Model::Hsl | Model::Hsb => (channel(a, 360.0, |v| v)?, channel(b, 100.0, |v| v)?, channel(c, 100.0, |v| v)?),
        Model::Rgb => (channel(a, 255.0, |v| v)?, channel(b, 255.0, |v| v)?, channel(c, 255.0, |v| v)?),
    };
    let alpha = match alpha.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => channel(s, 100.0, |v| v)?,
        None => Channel::Fixed(100.0),
    };
    Some((slot, Formula { model, ch: [a, b, c, alpha] }))
}

fn num_text(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// The formula as text (no slot).
pub fn format(f: &Formula) -> String {
    let places = |k: usize| match (f.model, k) {
        (Model::Oklch, 0 | 1) => 4,
        (_, 3) => 1,
        _ => 2,
    };
    let ch: Vec<String> = f
        .ch
        .iter()
        .enumerate()
        .map(|(k, c)| match c {
            Channel::Fixed(v) => num_text(*v, places(k)),
            Channel::Var(n) => format!("{{{n}}}"),
        })
        .collect();
    let base = format!("{}({} {} {}", f.model.name(), ch[0], ch[1], ch[2]);
    if matches!(f.ch[3], Channel::Fixed(a) if (a - 100.0).abs() < 0.05) { format!("{base})") } else { format!("{base} / {})", ch[3]) }
}

/// The formula `text` holds for `slot` (None: the line without a slot).
pub fn formula_in(text: &str, slot: Option<&str>) -> Option<Formula> {
    text.lines().filter_map(parse_line).find(|(s, _)| s.as_deref() == slot).map(|(_, f)| f)
}

/// Every formula line in `text`.
pub fn formulas_in(text: &str) -> Vec<(Option<String>, Formula)> {
    text.lines().filter_map(parse_line).collect()
}

/// `text` with `slot`'s formula replaced by `f` (added when missing, removed for None); other
/// lines (a description's own text, other slots) kept.
pub fn with_formula(text: &str, slot: Option<&str>, f: Option<&Formula>) -> String {
    let mut lines: Vec<String> =
        text.lines().filter(|l| !matches!(parse_line(l), Some((s, _)) if s.as_deref() == slot)).map(str::to_string).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if let Some(f) = f {
        lines.push(match slot {
            Some(s) => format!("{s} {}", format(f)),
            None => format(f),
        });
    }
    lines.join("\n")
}

// ---- where colours live --------------------------------------------------------------------------

/// The colour at `slot` of node `id` (a paint's alpha is its opacity).
pub fn node_color(doc: &Document, id: NodeId, slot: &str) -> Option<Color> {
    let n = doc.node(id)?;
    let parts: Vec<&str> = slot.split('/').collect();
    match parts.as_slice() {
        ["background"] => match &n.kind {
            NodeKind::Page { background, .. } => Some(*background),
            _ => None,
        },
        [list @ ("fills" | "strokes"), i, rest @ ..] => {
            let paints = if *list == "fills" { &n.fills } else { &n.strokes };
            let p = paints.get(i.parse::<usize>().ok()?)?;
            match (&p.kind, rest) {
                (PaintKind::Solid { color }, []) => Some(color.with_alpha(p.opacity)),
                (PaintKind::Gradient(g), ["stops", j]) => g.stops.get(j.parse::<usize>().ok()?).map(|s| s.color),
                _ => None,
            }
        }
        ["effects", i, rest @ ..] => n.effects.get(i.parse::<usize>().ok()?)?.color(rest == ["secondary"]),
        _ => None,
    }
}

/// Set the colour at `slot` of node `id`.
fn set_node_color(doc: &mut Document, id: NodeId, slot: &str, c: Color) -> Option<()> {
    let parts: Vec<String> = slot.split('/').map(str::to_string).collect();
    let n = doc.get_mut(id).ok()?;
    match parts.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["background"] => match &mut n.kind {
            NodeKind::Page { background, .. } => *background = c.with_alpha(1.0),
            _ => return None,
        },
        [list @ ("fills" | "strokes"), i, rest @ ..] => {
            let paints = if *list == "fills" { &mut n.fills } else { &mut n.strokes };
            let p = paints.get_mut(i.parse::<usize>().ok()?)?;
            match (&mut p.kind, rest) {
                (PaintKind::Solid { color }, []) => {
                    *color = c.with_alpha(1.0);
                    p.opacity = c.a;
                }
                (PaintKind::Gradient(g), ["stops", j]) => g.stops.get_mut(j.parse::<usize>().ok()?)?.color = c,
                _ => return None,
            }
        }
        ["effects", i, rest @ ..] => *n.effects.get_mut(i.parse::<usize>().ok()?)?.color_mut(rest == ["secondary"])? = c,
        _ => return None,
    }
    Some(())
}

/// The colour at `slot` of a colour style's paints ("paints/0", "paints/1/stops/2"; None = first).
fn style_color(kind: &StyleKind, slot: Option<&str>) -> Option<Color> {
    let StyleKind::Paint { paints } = kind else { return None };
    let parts: Vec<&str> = slot.unwrap_or("paints/0").split('/').collect();
    match parts.as_slice() {
        ["paints", i, rest @ ..] => {
            let p = paints.get(i.parse::<usize>().ok()?)?;
            match (&p.kind, rest) {
                (PaintKind::Solid { color }, []) => Some(color.with_alpha(p.opacity)),
                (PaintKind::Gradient(g), ["stops", j]) => g.stops.get(j.parse::<usize>().ok()?).map(|s| s.color),
                _ => None,
            }
        }
        _ => None,
    }
}

fn set_style_color(kind: &mut StyleKind, slot: Option<&str>, c: Color) -> Option<()> {
    let StyleKind::Paint { paints } = kind else { return None };
    let parts: Vec<&str> = slot.unwrap_or("paints/0").split('/').collect();
    match parts.as_slice() {
        ["paints", i, rest @ ..] => {
            let p = paints.get_mut(i.parse::<usize>().ok()?)?;
            match (&mut p.kind, rest) {
                (PaintKind::Solid { color }, []) => {
                    *color = c.with_alpha(1.0);
                    p.opacity = c.a;
                }
                (PaintKind::Gradient(g), ["stops", j]) => g.stops.get_mut(j.parse::<usize>().ok()?)?.color = c,
                _ => return None,
            }
        }
        _ => return None,
    }
    Some(())
}

fn same(a: Color, b: Color) -> bool {
    a.to_rgba8() == b.to_rgba8()
}

// ---- keeping colours in step ------------------------------------------------------------------------

/// After an edit: refix formulas whose colour was edited directly, then recompute every linked
/// colour that may have changed (all of them when variables changed, else the touched layers'
/// and styles'). Called by the engine's normalisation, inside the edit's undo step.
pub fn resolve(doc: &mut Document, before: &Document, touched: &BTreeSet<NodeId>) {
    let vars_changed = doc.variables != before.variables;
    let profile = doc.color_profile;
    resolve_variables(doc, before, vars_changed, profile);
    resolve_styles(doc, before, vars_changed, profile);
    // Layers (and pages) with formulas.
    // A frame switched mode (or a layer moved into another frame) changes what its layers read.
    let modes_changed = touched.iter().any(|id| doc.node(*id).map(|n| &n.var_modes) != before.node(*id).map(|n| &n.var_modes));
    let candidates: Vec<NodeId> = if vars_changed || modes_changed || doc.color_profile != before.color_profile {
        doc.ids().filter(|id| doc.node(*id).is_some_and(|n| n.plugin_data.contains_key(NODE_KEY))).collect()
    } else {
        touched.iter().copied().filter(|id| doc.node(*id).is_some_and(|n| n.plugin_data.contains_key(NODE_KEY))).collect()
    };
    for id in candidates {
        let Some(text) = doc.node(id).and_then(|n| n.plugin_data.get(NODE_KEY)).cloned() else { continue };
        let prev_text = before.node(id).and_then(|n| n.plugin_data.get(NODE_KEY)).cloned();
        let modes = doc.modes_for(id);
        let mut new_text = text.clone();
        for (slot, f) in formulas_in(&text) {
            let Some(slot) = slot else { continue };
            let Some(now) = node_color(doc, id, &slot) else { continue };
            let mut f = f;
            // Edited directly (same formula as before, the colour changed): follow the edit.
            let was = before.node(id).and_then(|_| node_color(before, id, &slot));
            let same_formula = prev_text.as_deref().and_then(|t| formula_in(t, Some(&slot))).as_ref() == Some(&f);
            if same_formula && was.is_some_and(|w| !same(w, now)) {
                f = f.refixed(now, profile);
                new_text = with_formula(&new_text, Some(&slot), Some(&f));
            }
            if let Some(c) = f.evaluate(doc, &modes)
                && !same(c, now)
            {
                let _ = set_node_color(doc, id, &slot, c);
            }
        }
        if new_text != text
            && let Ok(n) = doc.get_mut(id)
        {
            n.plugin_data.insert(NODE_KEY.into(), new_text);
        }
    }
}

fn resolve_variables(doc: &mut Document, before: &Document, vars_changed: bool, profile: ColorProfile) {
    if !vars_changed && doc.color_profile == before.color_profile {
        return;
    }
    let mut updates: Vec<(String, BTreeMap<String, VariableValue>, Option<String>)> = Vec::new();
    for v in doc.variables.variables.iter().filter(|v| v.kind == VariableType::Color) {
        let Some(mut f) = formula_in(&v.description, None) else { continue };
        let Some(coll) = doc.variables.collection(&v.collection) else { continue };
        let mut description = None;
        if let Some(prev) = before.variables.variables.iter().find(|p| p.id == v.id)
            && formula_in(&prev.description, None).as_ref() == Some(&f)
            && let Some(VariableValue::Color(c)) = coll.modes.iter().find_map(|m| match (v.values.get(&m.id), prev.values.get(&m.id)) {
                (Some(now), Some(was)) if now != was => Some(now.clone()),
                _ => None,
            })
        {
            f = f.refixed(c, profile);
            description = Some(with_formula(&v.description, None, Some(&f)));
        }
        let mut values = v.values.clone();
        for m in &coll.modes {
            // An alias in a mode is the user's choice: leave it.
            if matches!(values.get(&m.id), Some(VariableValue::Alias(_))) {
                continue;
            }
            let modes = BTreeMap::from([(coll.id.clone(), m.id.clone())]);
            if let Some(c) = f.evaluate(doc, &modes) {
                values.insert(m.id.clone(), VariableValue::Color(c));
            }
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

fn resolve_styles(doc: &mut Document, before: &Document, vars_changed: bool, profile: ColorProfile) {
    if !vars_changed && doc.styles == before.styles && doc.color_profile == before.color_profile {
        return;
    }
    let modes = BTreeMap::new();
    let mut updates = Vec::new();
    for (i, st) in doc.styles.iter().enumerate() {
        let lines = formulas_in(&st.description);
        if lines.is_empty() {
            continue;
        }
        let prev = before.styles.iter().find(|p| p.id == st.id);
        let mut kind = st.kind.clone();
        let mut description = st.description.clone();
        for (slot, f) in lines {
            let Some(now) = style_color(&kind, slot.as_deref()) else { continue };
            let mut f = f;
            if let Some(p) = prev
                && formula_in(&p.description, slot.as_deref()).as_ref() == Some(&f)
                && style_color(&p.kind, slot.as_deref()).is_some_and(|w| !same(w, now))
            {
                f = f.refixed(now, profile);
                description = with_formula(&description, slot.as_deref(), Some(&f));
            }
            if let Some(c) = f.evaluate(doc, &modes)
                && !same(c, now)
            {
                let _ = set_style_color(&mut kind, slot.as_deref(), c);
            }
        }
        if kind != st.kind || description != st.description {
            updates.push((i, kind, description));
        }
    }
    for (i, kind, description) in updates {
        if let Some(st) = doc.styles.get_mut(i) {
            st.kind = kind;
            st.description = description;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::Session;

    fn f(model: Model, ch: [Channel; 4]) -> Formula {
        Formula { model, ch }
    }

    #[test]
    fn lines_parse_and_format() {
        use Channel::{Fixed as X, Var as V};
        let (s, g) = parse_line("oklch(0.62 0.19 {Brand/Hue})").unwrap();
        assert_eq!((s, &g), (None, &f(Model::Oklch, [X(0.62), X(0.19), V("Brand/Hue".into()), X(100.0)])));
        assert_eq!(format(&g), "oklch(0.62 0.19 {Brand/Hue})");
        let (s, g) = parse_line("fills/0/stops/1 hsl({My hue} 70% 40 / {Op})").unwrap();
        assert_eq!(s.as_deref(), Some("fills/0/stops/1"));
        assert_eq!(g, f(Model::Hsl, [V("My hue".into()), X(70.0), X(40.0), V("Op".into())]));
        assert_eq!(format(&g), "hsl({My hue} 70 40 / {Op})");
        // Legacy forms: L as a percentage, C as % of 0.4.
        assert_eq!(parse_line("OKLCH(62% 50% 120deg)").unwrap().1, f(Model::Oklch, [X(0.62), X(0.2), X(120.0), X(100.0)]));
        assert_eq!(parse_line("rgb(255, 0, {G})").unwrap().1, f(Model::Rgb, [X(255.0), X(0.0), V("G".into()), X(100.0)]));
        assert!(parse_line("oklch(1 2)").is_none() && parse_line("oklch({x 1 2)").is_none() && parse_line("lab(1 2 3)").is_none());
        assert!(parse_line("Main brand colour").is_none());
        let text = "note\nfills/0 rgb(1 2 3)\noklch(0.1 0 0)";
        assert_eq!(with_formula(text, Some("fills/0"), None), "note\noklch(0.1 0 0)");
        assert_eq!(with_formula(text, None, Some(&g)), "note\nfills/0 rgb(1 2 3)\nhsl({My hue} 70 40 / {Op})");
        assert_eq!(formula_in(text, Some("fills/0")).map(|f| f.model), Some(Model::Rgb));
    }

    #[test]
    fn models_round_trip_through_their_values() {
        let c = snap(Color::from_hex("#3A81F6CC").unwrap_or_default());
        for m in [Model::Oklch, Model::Hsl, Model::Hsb, Model::Rgb] {
            for p in ColorProfile::ALL {
                assert_eq!(m.color(m.values(c, p), p).to_css_hex(), "#3A81F6CC", "{m:?} {p:?}");
            }
        }
    }

    fn id(v: serde_json::Value) -> u64 {
        v["id"].as_u64().unwrap()
    }

    fn fill(s: &Session, n: u64) -> Color {
        node_color(s.doc().unwrap(), NodeId(n), "fills/0").unwrap()
    }

    #[test]
    fn a_layer_fill_follows_its_variables_in_its_modes_and_its_own_edits() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Theme", "modes": ["Light", "Dark"]})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.create", &json!({"collection": c, "name": "Hue", "type": "number", "value": 280})).unwrap();
        s.execute("variable.create", &json!({"collection": c, "name": "Op", "type": "number", "values": {"Light": 100, "Dark": 50}})).unwrap();
        let frame = id(s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap());
        let r = id(s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 10, "height": 10, "parent": frame})).unwrap());
        s.execute("node.set", &json!({"ids": [r], "props": {"pluginData": {NODE_KEY: "fills/0 hsl({Hue} 80 50 / {Op})"}}})).unwrap();
        let got = fill(&s, r);
        assert!((Hsl::from_color(got).h - 280.0).abs() < 1.0 && got.a == 1.0, "{got:?}");
        // The frame in Dark: the fill's opacity follows Op there.
        s.execute("variable.setMode", &json!({"ids": [frame], "collection": c, "mode": "Dark"})).unwrap();
        assert!((fill(&s, r).a - 0.5).abs() < 0.01);
        // A variable change restyles it, in one undo step.
        s.execute("variable.set", &json!({"variable": "Hue", "mode": "Dark", "value": 130})).unwrap();
        assert!((Hsl::from_color(fill(&s, r)).h - 130.0).abs() < 1.0);
        s.execute("edit.undo", &json!({})).unwrap();
        assert!((Hsl::from_color(fill(&s, r)).h - 280.0).abs() < 1.0);
        // Editing the fill itself refixes the fixed channels (S, L) and keeps the links.
        s.execute("fill.set", &json!({"ids": [r], "index": 0, "color": "#00FF00"})).unwrap();
        let text = s.doc().unwrap().node(NodeId(r)).unwrap().plugin_data.get(NODE_KEY).cloned().unwrap();
        let g = formula_in(&text, Some("fills/0")).unwrap();
        assert_eq!((g.var(0), g.var(3)), (Some("Hue"), Some("Op")));
        assert!(matches!(g.ch[1], Channel::Fixed(v) if (v - 100.0).abs() < 0.5), "{g:?}");
        assert!((Hsl::from_color(fill(&s, r)).h - 280.0).abs() < 1.0, "the hue stays linked");
    }

    #[test]
    fn variables_styles_gradients_effects_and_pages_follow_too() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Theme", "modes": ["Light", "Dark"]})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.create", &json!({"collection": c, "name": "Hue", "type": "number", "value": 280})).unwrap();
        s.execute("variable.create", &json!({"collection": c, "name": "Lightness", "type": "number", "values": {"Light": 62, "Dark": 80}})).unwrap();
        let p = s.execute("variable.create", &json!({"collection": c, "name": "Primary", "type": "color", "value": "#000000"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.update", &json!({"variable": p, "description": "Brand\noklch({Lightness} 0.15 {Hue})"})).unwrap();
        let st = s.execute("style.create", &json!({"type": "paint", "name": "Accent", "value": "#000000"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("style.update", &json!({"style": st, "description": "paints/0 oklch(0.7 0.1 {Hue})"})).unwrap();
        let r = id(s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
        s.execute("fill.set", &json!({"ids": [r], "index": 0, "paint": {"type": "gradient", "kind": "linear", "stops": [{"position": 0, "color": "#000000"}, {"position": 1, "color": "#FFFFFF"}], "from": [0, 0], "to": [1, 1], "width": [0, 1]}})).unwrap();
        s.execute("effect.add", &json!({"ids": [r], "type": "dropShadow"})).unwrap();
        let page = s.page().unwrap().0;
        s.execute("node.set", &json!({"ids": [r], "props": {"pluginData": {NODE_KEY: "fills/0/stops/1 oklch(0.5 0.1 {Hue})\neffects/0 rgb(0 0 {Hue} / 50)"}}})).unwrap();
        s.execute("node.set", &json!({"ids": [page], "props": {"pluginData": {NODE_KEY: "background hsb({Hue} 10 95)"}}})).unwrap();
        s.execute("variable.set", &json!({"variable": "Hue", "mode": "Light", "value": 130})).unwrap();
        let doc = s.doc().unwrap();
        let prof = doc.color_profile;
        let hue = |c: Color| Oklch::from_color(c, prof).h;
        let light = doc.variables.collection(&c).unwrap().find_mode("Light").unwrap().id.clone();
        let VariableValue::Color(pc) = doc.variables.get("Primary").unwrap().values.get(&light).cloned().unwrap() else { panic!() };
        assert!((hue(pc) - 130.0).abs() < 1.0 && (Oklch::from_color(pc, prof).l - 0.62).abs() < 0.01);
        assert!((hue(style_color(&doc.style(&st).unwrap().kind, None).unwrap()) - 130.0).abs() < 1.0);
        assert!((hue(node_color(doc, NodeId(r), "fills/0/stops/1").unwrap()) - 130.0).abs() < 1.0);
        assert!(same(node_color(doc, NodeId(r), "fills/0/stops/0").unwrap(), Color::BLACK), "an unlinked stop stays");
        let e = node_color(doc, NodeId(r), "effects/0").unwrap();
        assert_eq!((e.to_rgba8()[2], (e.a * 100.0).round()), (130, 50.0));
        assert!((Hsb::from_color(node_color(doc, NodeId(page), "background").unwrap()).h - 130.0).abs() < 1.5);
        // A missing variable leaves the colour as it was.
        s.execute("variable.update", &json!({"variable": p, "description": "oklch(0.5 0.1 {Nope})"})).unwrap();
        let doc = s.doc().unwrap();
        let VariableValue::Color(again) = doc.variables.get("Primary").unwrap().values.get(&light).cloned().unwrap() else { panic!() };
        assert!(same(again, pc));
    }
}
