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
//! Derived colours (requested in storytold/figcraft#1280), in CSS's own syntax:
//!
//! ```text
//! oklch(from {Brand/Primary} l c h / 50%)          the base colour, half transparent
//! oklch(from {Brand/Primary} calc(l - 0.08) c h)   a darker hover
//! color-mix(in oklch, {Brand/Primary} 70%, {Surface})   a mix of two colours
//! calc({Space/Base} * 2)                           a number variable from another one
//! ```
//!
//! `from` takes a colour variable (`{name}`) or a hex colour; its channels are then the keywords
//! of the model (`l c h`, `h s l`, `h s b`, `r g b` and `alpha`), usable in `calc()` with
//! numbers and number variables. `color-mix` mixes in `oklch`, `hsl`, `hsb` or `srgb`; editing a
//! mixed colour (or a calculated number) by hand detaches it from its formula.
//!
//! Units are the colour picker's: OKLCH L 0–1 (a variable above 1 is read as a percentage), C 0–0.4,
//! H degrees; HSL / HSB H degrees, S L B 0–100; RGB 0–255; alpha 0–100. Fixed values also take
//! `%` (OKLCH L `62%`, C `50%` of 0.4). A variable is found by name (or id) and read in the modes
//! that apply where the colour is (a layer's frames, a colour variable's own collection mode).
//! Editing the colour itself (hex, picker, eyedropper) rewrites the formula's fixed channels from
//! it; linked channels stay linked. Without FigCraft Plus the colours simply keep their last values.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_color::oklch_profile::Oklch;
use figcraft_color::{Color, ColorProfile, Hsb, Hsl, OKLCH_FULL_CHROMA, PaintKind};
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

/// One channel: a fixed value (picker units), a number variable's name, or an expression (a
/// keyword of the `from` colour, `calc(…)`).
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Fixed(f64),
    Var(String),
    Expr(Expr),
}

/// Arithmetic in a formula: numbers, number variables and the `from` colour's channels.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    /// A channel of the `from` colour in the formula's model (alpha is 3), in picker units.
    Key(usize),
    Neg(Box<Expr>),
    Bin(Box<Expr>, char, Box<Expr>),
}

/// The colour a derived formula starts from: a colour variable (by name or id) or a fixed one.
#[derive(Clone, Debug, PartialEq)]
pub enum Base {
    Var(String),
    Color(Color),
}

/// How a formula derives its colour from other colours.
#[derive(Clone, Debug, PartialEq)]
pub enum Derive {
    /// `model(from base …)`: the channels may use the base colour's (CSS relative colour syntax).
    From(Base),
    /// `color-mix(in model, a [pa%], b [pb%])`.
    Mix { a: Base, pa: Option<f64>, b: Base, pb: Option<f64> },
}

/// A colour formula: its model, three channels and alpha, and what it derives from.
#[derive(Clone, Debug, PartialEq)]
pub struct Formula {
    pub model: Model,
    /// The model's three channels, then alpha (unused by a mix).
    pub ch: [Channel; 4],
    pub derive: Option<Derive>,
}

impl Formula {
    /// Every channel fixed to `c`'s values.
    pub fn fixed(model: Model, c: Color, profile: ColorProfile) -> Formula {
        let v = model.values(c, profile);
        Formula { model, ch: v.map(Channel::Fixed), derive: None }
    }

    /// Whether the colour is a mix: edited by hand, it leaves its formula.
    pub fn is_mix(&self) -> bool {
        matches!(self.derive, Some(Derive::Mix { .. }))
    }

    /// The colour variables it derives from.
    pub fn bases(&self) -> Vec<&str> {
        match &self.derive {
            Some(Derive::From(Base::Var(n))) => vec![n.as_str()],
            Some(Derive::Mix { a, b, .. }) => [a, b].into_iter().filter_map(|x| if let Base::Var(n) = x { Some(n.as_str()) } else { None }).collect(),
            _ => Vec::new(),
        }
    }

    /// Every variable it reads (number variables in its channels, colour variables it derives from).
    pub fn reads(&self) -> Vec<String> {
        let mut out: Vec<String> = self.bases().into_iter().map(str::to_string).collect();
        for c in &self.ch {
            match c {
                Channel::Var(n) => out.push(n.clone()),
                Channel::Expr(e) => e.vars(&mut out),
                Channel::Fixed(_) => {}
            }
        }
        out
    }

    /// Whether any channel is linked to a variable, or the colour derives from others.
    pub fn linked(&self) -> bool {
        self.derive.is_some() || self.ch.iter().any(|c| matches!(c, Channel::Var(_) | Channel::Expr(_)))
    }

    /// The variable channel `k` is linked to.
    pub fn var(&self, k: usize) -> Option<&str> {
        match self.ch.get(k)? {
            Channel::Var(n) => Some(n),
            Channel::Fixed(_) | Channel::Expr(_) => None,
        }
    }

    /// What channel `k` shows when it isn't a plain number: its variable's name or its expression.
    pub fn channel_label(&self, k: usize) -> Option<String> {
        match self.ch.get(k)? {
            Channel::Var(n) => Some(n.clone()),
            Channel::Expr(e) => Some(expr_text(e, &keys(self.model))),
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
        Formula { model: self.model, ch, derive: self.derive.clone() }
    }

    /// The colour in `modes` (None when a linked variable is missing or not a number).
    pub fn evaluate(&self, doc: &Document, modes: &BTreeMap<String, String>) -> Option<Color> {
        let profile = doc.color_profile;
        let base = match &self.derive {
            Some(Derive::Mix { a, pa, b, pb }) => {
                let ca = base_color(doc, a, modes)?;
                let cb = base_color(doc, b, modes)?;
                return Some(mix(self.model, ca, cb, weights(*pa, *pb)?, profile));
            }
            Some(Derive::From(b)) => Some(self.model.values(base_color(doc, b, modes)?, profile)),
            None => None,
        };
        let mut v = [0.0; 4];
        for (k, c) in self.ch.iter().enumerate() {
            let x = match c {
                Channel::Fixed(x) => *x,
                Channel::Var(n) => {
                    let x = number(doc, n, modes)?;
                    // OKLCH lightness from a variable: 0–1, or a percentage above 1.
                    if self.model == Model::Oklch && k == 0 && x > 1.0 { x / 100.0 } else { x }
                }
                Channel::Expr(e) => e.eval(doc, modes, base.as_ref())?,
            };
            if let Some(slot) = v.get_mut(k) {
                *slot = x;
            }
        }
        Some(self.model.color(v, doc.color_profile))
    }
}

impl Expr {
    /// Its value: number variables read in `modes`, keywords from `base` (the `from` colour).
    pub fn eval(&self, doc: &Document, modes: &BTreeMap<String, String>, base: Option<&[f64; 4]>) -> Option<f64> {
        let v = match self {
            Expr::Num(x) => *x,
            Expr::Var(n) => number(doc, n, modes)?,
            Expr::Key(k) => *base?.get(*k)?,
            Expr::Neg(e) => -e.eval(doc, modes, base)?,
            Expr::Bin(a, op, b) => {
                let (a, b) = (a.eval(doc, modes, base)?, b.eval(doc, modes, base)?);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' if b != 0.0 => a / b,
                    _ => return None,
                }
            }
        };
        v.is_finite().then_some(v)
    }

    fn vars(&self, out: &mut Vec<String>) {
        match self {
            Expr::Var(n) => out.push(n.clone()),
            Expr::Neg(e) => e.vars(out),
            Expr::Bin(a, _, b) => {
                a.vars(out);
                b.vars(out);
            }
            Expr::Num(_) | Expr::Key(_) => {}
        }
    }

    /// Whether it reads the `from` colour.
    fn uses_keys(&self) -> bool {
        match self {
            Expr::Key(_) => true,
            Expr::Neg(e) => e.uses_keys(),
            Expr::Bin(a, _, b) => a.uses_keys() || b.uses_keys(),
            Expr::Num(_) | Expr::Var(_) => false,
        }
    }
}

/// The colour `b` stands for in `modes`.
fn base_color(doc: &Document, b: &Base, modes: &BTreeMap<String, String>) -> Option<Color> {
    match b {
        Base::Color(c) => Some(*c),
        Base::Var(n) => {
            let v = doc.variables.get(n)?;
            if v.kind != VariableType::Color {
                return None;
            }
            match doc.variables.resolve(&v.id, modes)? {
                VariableValue::Color(c) => Some(c),
                _ => None,
            }
        }
    }
}

/// `color-mix` weights of the two colours (CSS rules: one percentage gives the other the rest; two
/// are scaled to add up to 100%).
fn weights(pa: Option<f64>, pb: Option<f64>) -> Option<(f64, f64)> {
    let (a, b) = match (pa, pb) {
        (None, None) => (50.0, 50.0),
        (Some(a), None) => (a, 100.0 - a),
        (None, Some(b)) => (100.0 - b, b),
        (Some(a), Some(b)) => (a, b),
    };
    let sum = a + b;
    (a >= 0.0 && b >= 0.0 && sum > 0.0).then(|| (a / sum, b / sum))
}

/// Mix `a` and `b` (weights `wa + wb = 1`) in `model`: hues the shorter way round, a grey taking
/// the other colour's hue.
fn mix(model: Model, a: Color, b: Color, (wa, wb): (f64, f64), profile: ColorProfile) -> Color {
    let va = model.values(a, profile);
    let vb = model.values(b, profile);
    let mut v = [0.0; 4];
    for k in 0..4 {
        v[k] = va[k] * wa + vb[k] * wb;
    }
    // Which channel is the hue, and which says whether there is one.
    let hue = match model {
        Model::Oklch => Some((2, 1, 0.0005)),
        Model::Hsl | Model::Hsb => Some((0, 1, 0.05)),
        Model::Rgb => None,
    };
    if let Some((h, sat, eps)) = hue {
        let (ha, hb) = match (va[sat] < eps, vb[sat] < eps) {
            (true, false) => (vb[h], vb[h]),
            (false, true) => (va[h], va[h]),
            _ => (va[h], vb[h]),
        };
        let mut d = hb - ha;
        if d > 180.0 {
            d -= 360.0;
        } else if d < -180.0 {
            d += 360.0;
        }
        v[h] = (ha + d * wb).rem_euclid(360.0);
    }
    model.color(v, profile)
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

/// Split on whitespace (and commas) outside braces and brackets: variable names may hold spaces,
/// `calc(…)` holds its own.
fn split_channels(s: &str) -> Option<Vec<String>> {
    split_top(s, |c| c.is_whitespace() || c == ',')
}

/// Split `s` where `sep` matches outside braces and brackets.
fn split_top(s: &str, sep: impl Fn(char) -> bool) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut braces, mut parens) = (0usize, 0usize);
    for ch in s.chars() {
        match ch {
            '{' => braces += 1,
            '}' => braces = braces.checked_sub(1)?,
            '(' if braces == 0 => parens += 1,
            ')' if braces == 0 => parens = parens.checked_sub(1)?,
            _ => {}
        }
        if sep(ch) && braces == 0 && parens == 0 {
            if !cur.trim().is_empty() {
                out.push(std::mem::take(&mut cur).trim().to_string());
            }
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if braces != 0 || parens != 0 {
        return None;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    Some(out)
}

/// The channels and the alpha of a formula's inside: alpha follows a `/` outside braces and
/// brackets (`{Brand/Hue}` and `calc(l / 2)` keep theirs).
fn split_alpha(inner: &str) -> Option<(&str, Option<&str>)> {
    let (mut braces, mut parens) = (0usize, 0usize);
    for (i, ch) in inner.char_indices() {
        match ch {
            '{' => braces += 1,
            '}' => braces = braces.checked_sub(1)?,
            '(' if braces == 0 => parens += 1,
            ')' if braces == 0 => parens = parens.checked_sub(1)?,
            '/' if braces == 0 && parens == 0 => return Some((inner.get(..i)?, inner.get(i + 1..))),
            _ => {}
        }
    }
    Some((inner, None))
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

/// A channel of a relative colour (`model(from …)`): a keyword of the base colour, `calc(…)`, or
/// what [`channel`] reads.
fn rel_channel(token: &str, keys: &[&str; 4], pct: f64, fix: impl Fn(f64) -> f64) -> Option<Channel> {
    let t = token.trim();
    if let Some(inner) = t.strip_prefix("calc(").and_then(|r| r.strip_suffix(')')) {
        return parse_expr(inner, Some(keys)).map(Channel::Expr);
    }
    if let Some(k) = keys.iter().position(|k| t.eq_ignore_ascii_case(k)) {
        return Some(Channel::Expr(Expr::Key(k)));
    }
    channel(t, pct, fix)
}

/// Parse arithmetic: numbers (a `%` is a hundredth), `{variables}`, `keys` (the base colour's
/// channels) and `+ - * /` with brackets.
pub fn parse_expr(s: &str, keys: Option<&[&str; 4]>) -> Option<Expr> {
    struct P<'a> {
        s: &'a [u8],
        i: usize,
        keys: Option<&'a [&'a str; 4]>,
        depth: usize,
    }
    impl P<'_> {
        fn space(&mut self) {
            while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
                self.i += 1;
            }
        }
        fn eat(&mut self, c: u8) -> bool {
            self.space();
            if self.s.get(self.i) == Some(&c) {
                self.i += 1;
                true
            } else {
                false
            }
        }
        fn sum(&mut self) -> Option<Expr> {
            let mut v = self.product()?;
            loop {
                if self.eat(b'+') {
                    v = Expr::Bin(Box::new(v), '+', Box::new(self.product()?));
                } else if self.eat(b'-') {
                    v = Expr::Bin(Box::new(v), '-', Box::new(self.product()?));
                } else {
                    return Some(v);
                }
            }
        }
        fn product(&mut self) -> Option<Expr> {
            let mut v = self.factor()?;
            loop {
                if self.eat(b'*') {
                    v = Expr::Bin(Box::new(v), '*', Box::new(self.factor()?));
                } else if self.eat(b'/') {
                    v = Expr::Bin(Box::new(v), '/', Box::new(self.factor()?));
                } else {
                    return Some(v);
                }
            }
        }
        fn factor(&mut self) -> Option<Expr> {
            self.depth += 1;
            if self.depth > 64 {
                return None;
            }
            self.space();
            if self.eat(b'-') {
                return Some(Expr::Neg(Box::new(self.factor()?)));
            }
            if self.eat(b'(') {
                let v = self.sum()?;
                return self.eat(b')').then_some(v);
            }
            if self.eat(b'{') {
                let start = self.i;
                while self.s.get(self.i).is_some_and(|c| *c != b'}') {
                    self.i += 1;
                }
                let name = std::str::from_utf8(self.s.get(start..self.i)?).ok()?.trim().to_string();
                self.i += 1;
                return (!name.is_empty()).then_some(Expr::Var(name));
            }
            let start = self.i;
            while self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
                self.i += 1;
            }
            if self.i > start {
                let v = std::str::from_utf8(self.s.get(start..self.i)?).ok()?.parse::<f64>().ok().filter(|v| v.is_finite())?;
                // Units after a number: `%` a hundredth, `deg` / `px` nothing.
                if self.s.get(self.i) == Some(&b'%') {
                    self.i += 1;
                    return Some(Expr::Num(v / 100.0));
                }
                while self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
                    self.i += 1;
                }
                return Some(Expr::Num(v));
            }
            while self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
                self.i += 1;
            }
            let word = std::str::from_utf8(self.s.get(start..self.i)?).ok()?;
            let k = self.keys?.iter().position(|k| word.eq_ignore_ascii_case(k))?;
            Some(Expr::Key(k))
        }
    }
    let mut p = P { s: s.as_bytes(), i: 0, keys, depth: 0 };
    let v = p.sum()?;
    p.space();
    (p.i == s.len()).then_some(v)
}

/// The keywords naming a model's channels in a relative colour.
fn keys(model: Model) -> [&'static str; 4] {
    match model {
        Model::Oklch => ["l", "c", "h", "alpha"],
        Model::Hsl => ["h", "s", "l", "alpha"],
        Model::Hsb => ["h", "s", "b", "alpha"],
        Model::Rgb => ["r", "g", "b", "alpha"],
    }
}

/// A base colour: `{Colour variable}` or a hex colour.
fn base(token: &str) -> Option<Base> {
    let t = token.trim();
    if let Some(name) = t.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        let name = name.trim();
        return (!name.is_empty()).then(|| Base::Var(name.to_string()));
    }
    t.starts_with('#').then(|| Color::from_hex(t)).flatten().map(Base::Color)
}

/// `color-mix(in model, a [p%], b [q%])`'s inside.
fn parse_mix(inner: &str) -> Option<Formula> {
    let parts = split_top(inner, |c| c == ',')?;
    let [space, a, b] = parts.as_slice() else { return None };
    let model = space.trim().strip_prefix("in ")?.trim();
    let model = if model.eq_ignore_ascii_case("srgb") { Model::Rgb } else { Model::from_name(model)? };
    let side = |s: &str| -> Option<(Base, Option<f64>)> {
        let toks = split_channels(s)?;
        match toks.as_slice() {
            [c] => Some((base(c)?, None)),
            [c, p] | [p, c] if p.ends_with('%') => Some((base(c)?, Some(p.trim_end_matches('%').trim().parse::<f64>().ok().filter(|v| v.is_finite())?))),
            _ => None,
        }
    };
    let (a, pa) = side(a)?;
    let (b, pb) = side(b)?;
    weights(pa, pb)?;
    Some(Formula { model, ch: [0.0, 0.0, 0.0, 100.0].map(Channel::Fixed), derive: Some(Derive::Mix { a, pa, b, pb }) })
}

/// Parse a formula line: `(slot, formula)`.
pub fn parse_line(line: &str) -> Option<(Option<String>, Formula)> {
    let t = line.trim();
    let open = t.find('(')?;
    let head = t.get(..open)?.trim();
    let (slot, name) = match head.rsplit_once(char::is_whitespace) {
        Some((s, m)) => (Some(s.trim().to_string()), m),
        None => (None, head),
    };
    if slot.as_deref().is_some_and(|s| s.is_empty() || s.contains(char::is_whitespace)) {
        return None;
    }
    let inner = t.get(open + 1..)?.strip_suffix(')')?;
    if name.eq_ignore_ascii_case("color-mix") {
        return Some((slot, parse_mix(inner)?));
    }
    let model = Model::from_name(name)?;
    let (from, inner) = match inner.trim_start().strip_prefix("from ") {
        Some(rest) => {
            let rest = rest.trim_start();
            let end = if rest.starts_with('{') { rest.find('}')? + 1 } else { rest.find(char::is_whitespace)? };
            (Some(base(rest.get(..end)?)?), rest.get(end..)?)
        }
        None => (None, inner),
    };
    let (main, alpha) = split_alpha(inner)?;
    let parts = split_channels(main)?;
    let [a, b, c] = parts.as_slice() else { return None };
    let keys = keys(model);
    let tok = |s: &str, pct: f64, fix: &dyn Fn(f64) -> f64| if from.is_some() { rel_channel(s, &keys, pct, fix) } else { channel(s, pct, fix) };
    let id = |v: f64| v;
    let (a, b, c) = match model {
        // L: 0–1, `62%`, or a bare 62 meaning percent. C: `50%` of 0.4.
        Model::Oklch => (tok(a, 1.0, &|v| if v > 1.0 { v / 100.0 } else { v })?, tok(b, OKLCH_FULL_CHROMA, &id)?, tok(c, 360.0, &id)?),
        Model::Hsl | Model::Hsb => (tok(a, 360.0, &id)?, tok(b, 100.0, &id)?, tok(c, 100.0, &id)?),
        Model::Rgb => (tok(a, 255.0, &id)?, tok(b, 255.0, &id)?, tok(c, 255.0, &id)?),
    };
    let alpha = match alpha.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => tok(s, 100.0, &id)?,
        // A relative colour keeps its base's alpha unless it says otherwise.
        None if from.is_some() => Channel::Expr(Expr::Key(3)),
        None => Channel::Fixed(100.0),
    };
    // Keywords only mean something with a base colour.
    if from.is_none() && [&a, &b, &c, &alpha].iter().any(|ch| matches!(ch, Channel::Expr(e) if e.uses_keys())) {
        return None;
    }
    Some((slot, Formula { model, ch: [a, b, c, alpha], derive: from.map(Derive::From) }))
}

/// A number variable's formula in its description: a `calc(…)` line of numbers and number
/// variables.
pub fn number_formula_in(text: &str) -> Option<Expr> {
    text.lines().find_map(|l| l.trim().strip_prefix("calc(").and_then(|r| r.strip_suffix(')')).and_then(|e| parse_expr(e, None)))
}

/// `text` without its `calc(…)` line.
pub fn without_number_formula(text: &str) -> String {
    text.lines().filter(|l| !(l.trim().starts_with("calc(") && number_formula_in(l).is_some())).collect::<Vec<_>>().join("\n")
}

fn num_text(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// The formula as text (no slot).
pub fn format(f: &Formula) -> String {
    let base_text = |b: &Base| match b {
        Base::Var(n) => format!("{{{n}}}"),
        Base::Color(c) => c.to_css_hex(),
    };
    if let Some(Derive::Mix { a, pa, b, pb }) = &f.derive {
        let side = |b: &Base, p: &Option<f64>| match p {
            Some(p) => format!("{} {}%", base_text(b), num_text(*p, 2)),
            None => base_text(b),
        };
        let space = if f.model == Model::Rgb { "srgb" } else { f.model.name() };
        return format!("color-mix(in {space}, {}, {})", side(a, pa), side(b, pb));
    }
    let keys = keys(f.model);
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
            Channel::Expr(Expr::Key(i)) => keys.get(*i).copied().unwrap_or("alpha").to_string(),
            Channel::Expr(e) => format!("calc({})", expr_text(e, &keys)),
        })
        .collect();
    let from = match &f.derive {
        Some(Derive::From(b)) => format!("from {} ", base_text(b)),
        _ => String::new(),
    };
    let base = format!("{}({from}{} {} {}", f.model.name(), ch[0], ch[1], ch[2]);
    let plain_alpha = match &f.ch[3] {
        Channel::Fixed(a) => f.derive.is_none() && (a - 100.0).abs() < 0.05,
        Channel::Expr(Expr::Key(3)) => f.derive.is_some(),
        _ => false,
    };
    if plain_alpha { format!("{base})") } else { format!("{base} / {})", ch[3]) }
}

/// An expression as text.
pub fn expr_text(e: &Expr, keys: &[&str; 4]) -> String {
    match e {
        Expr::Num(v) => num_text(*v, 4),
        Expr::Var(n) => format!("{{{n}}}"),
        Expr::Key(k) => keys.get(*k).copied().unwrap_or("alpha").to_string(),
        Expr::Neg(e) => format!("-{}", expr_text(e, keys)),
        Expr::Bin(a, op, b) => {
            let side = |x: &Expr, right: bool| {
                let s = expr_text(x, keys);
                let low = matches!(x, Expr::Bin(_, o, _) if matches!(o, '+' | '-'));
                let need = low && (matches!(op, '*' | '/') || (right && *op == '-'));
                if need { format!("({s})") } else { s }
            };
            format!("{} {op} {}", side(a, false), side(b, true))
        }
    }
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
            if same_formula && was.is_some_and(|w| !same(w, now)) && f.evaluate(doc, &modes).is_none_or(|x| !same(x, now)) {
                if f.is_mix() {
                    // A mix edited by hand leaves its formula.
                    new_text = with_formula(&new_text, Some(&slot), None);
                    continue;
                }
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
            if new_text.trim().is_empty() {
                n.plugin_data.remove(NODE_KEY);
            } else {
                n.plugin_data.insert(NODE_KEY.into(), new_text);
            }
        }
    }
}

/// A variable's formula: a colour one, or a number one (`calc(…)`).
enum VarFormula {
    Color(Formula),
    Number(Expr),
}

impl VarFormula {
    fn reads(&self) -> Vec<String> {
        match self {
            VarFormula::Color(f) => f.reads(),
            VarFormula::Number(e) => {
                let mut out = Vec::new();
                e.vars(&mut out);
                out
            }
        }
    }

    fn of(v: &figcraft_doc::Variable) -> Option<VarFormula> {
        match v.kind {
            VariableType::Color => formula_in(&v.description, None).map(VarFormula::Color),
            VariableType::Number => number_formula_in(&v.description).map(VarFormula::Number),
            _ => None,
        }
    }
}

/// The ids of variables whose formulas read themselves, directly or through others (they are left
/// as they are).
fn cyclic(doc: &Document, formulas: &BTreeMap<String, VarFormula>) -> BTreeSet<String> {
    let id_of = |name: &str| doc.variables.get(name).map(|v| v.id.clone());
    let deps: BTreeMap<String, Vec<String>> =
        formulas.iter().map(|(id, f)| (id.clone(), f.reads().iter().filter_map(|n| id_of(n)).filter(|d| formulas.contains_key(d)).collect())).collect();
    let mut bad = BTreeSet::new();
    for start in deps.keys() {
        // Depth-first from `start`: reaching it again is a loop.
        let mut stack: Vec<&String> = deps.get(start).map(|d| d.iter().collect()).unwrap_or_default();
        let mut seen = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if n == start {
                bad.insert(start.clone());
                break;
            }
            if seen.insert(n.clone()) {
                stack.extend(deps.get(n).into_iter().flatten());
            }
        }
    }
    bad
}

fn resolve_variables(doc: &mut Document, before: &Document, vars_changed: bool, profile: ColorProfile) {
    if !vars_changed && doc.color_profile == before.color_profile {
        return;
    }
    // Formulas edited by hand: a colour's fixed channels follow the edit (a mix, or a calculated
    // number, leaves its formula).
    let mut descriptions: Vec<(String, String)> = Vec::new();
    for v in &doc.variables.variables {
        let Some(f) = VarFormula::of(v) else { continue };
        let Some(prev) = before.variables.variables.iter().find(|p| p.id == v.id) else { continue };
        let same_formula = match (&f, VarFormula::of(prev)) {
            (VarFormula::Color(a), Some(VarFormula::Color(b))) => *a == b,
            (VarFormula::Number(a), Some(VarFormula::Number(b))) => *a == b,
            _ => false,
        };
        let Some(coll) = doc.variables.collection(&v.collection) else { continue };
        // A value the formula gives anyway (an undo, a redo) isn't an edit.
        let edited = coll.modes.iter().find_map(|m| match (v.values.get(&m.id), prev.values.get(&m.id)) {
            (Some(now), Some(was)) if now != was && !matches!(now, VariableValue::Alias(_)) => {
                let modes = BTreeMap::from([(coll.id.clone(), m.id.clone())]);
                let gives = match (&f, now) {
                    (VarFormula::Color(f), VariableValue::Color(c)) => f.evaluate(doc, &modes).is_some_and(|x| same(x, *c)),
                    (VarFormula::Number(e), VariableValue::Number(x)) => e.eval(doc, &modes, None).is_some_and(|y| (x - y).abs() < 1e-6),
                    _ => false,
                };
                (!gives).then(|| now.clone())
            }
            _ => None,
        });
        let (true, Some(now)) = (same_formula, edited) else { continue };
        let description = match (&f, now) {
            (VarFormula::Color(f), VariableValue::Color(c)) if !f.is_mix() => with_formula(&v.description, None, Some(&f.refixed(c, profile))),
            (VarFormula::Color(_), _) => with_formula(&v.description, None, None),
            (VarFormula::Number(_), _) => without_number_formula(&v.description),
        };
        descriptions.push((v.id.clone(), description));
    }
    for (id, d) in descriptions {
        if let Some(v) = doc.variables.variables.iter_mut().find(|v| v.id == id) {
            v.description = d;
        }
    }
    // Then every formula, again while values change (a variable derived from a derived one), at
    // most a few rounds; loops are left alone.
    let formulas: BTreeMap<String, VarFormula> = doc.variables.variables.iter().filter_map(|v| Some((v.id.clone(), VarFormula::of(v)?))).collect();
    if formulas.is_empty() {
        return;
    }
    let skip = cyclic(doc, &formulas);
    for _ in 0..8 {
        let mut updates: Vec<(String, BTreeMap<String, VariableValue>)> = Vec::new();
        for v in &doc.variables.variables {
            if skip.contains(&v.id) {
                continue;
            }
            let Some(f) = formulas.get(&v.id) else { continue };
            let Some(coll) = doc.variables.collection(&v.collection) else { continue };
            let mut values = v.values.clone();
            for m in &coll.modes {
                // An alias in a mode is the user's choice: leave it.
                if matches!(values.get(&m.id), Some(VariableValue::Alias(_))) {
                    continue;
                }
                let modes = BTreeMap::from([(coll.id.clone(), m.id.clone())]);
                let value = match f {
                    VarFormula::Color(f) => f.evaluate(doc, &modes).map(VariableValue::Color),
                    VarFormula::Number(e) => e.eval(doc, &modes, None).map(|x| VariableValue::Number((x * 1e6).round() / 1e6)),
                };
                if let Some(value) = value {
                    values.insert(m.id.clone(), value);
                }
            }
            if values != v.values {
                updates.push((v.id.clone(), values));
            }
        }
        if updates.is_empty() {
            break;
        }
        for (id, values) in updates {
            if let Some(v) = doc.variables.variables.iter_mut().find(|v| v.id == id) {
                v.values = values;
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
                && f.evaluate(doc, &modes).is_none_or(|x| !same(x, now))
            {
                if f.is_mix() {
                    description = with_formula(&description, slot.as_deref(), None);
                    continue;
                }
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
        Formula { model, ch, derive: None }
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

    #[test]
    fn derived_formulas_parse_and_format() {
        // storytold/figcraft#1280: relative colours, mixes and calculated numbers.
        let (s, g) = parse_line("oklch(from {Brand/Primary} l c h / 50%)").unwrap();
        assert_eq!(s, None);
        assert_eq!(g.derive, Some(Derive::From(Base::Var("Brand/Primary".into()))));
        assert_eq!(g.ch[0], Channel::Expr(Expr::Key(0)));
        assert_eq!(g.ch[3], Channel::Fixed(50.0));
        assert_eq!(format(&g), "oklch(from {Brand/Primary} l c h / 50)");
        let (_, g) = parse_line("fills/0 oklch(from {Primary} calc(l - 0.08) c calc(h + 180))").unwrap();
        assert_eq!(format(&g), "oklch(from {Primary} calc(l - 0.08) c calc(h + 180))");
        assert_eq!(g.ch[3], Channel::Expr(Expr::Key(3)), "no alpha: the base's");
        let (_, g) = parse_line("hsl(from #FF0000 h s calc(l * {Shade}))").unwrap();
        assert_eq!(g.derive, Some(Derive::From(Base::Color(Color::from_hex("#FF0000").unwrap()))));
        assert_eq!(g.reads(), vec!["Shade".to_string()]);
        let (_, g) = parse_line("color-mix(in oklch, {Primary} 70%, {Surface})").unwrap();
        assert!(g.is_mix() && g.linked());
        assert_eq!(format(&g), "color-mix(in oklch, {Primary} 70%, {Surface})");
        assert_eq!(parse_line("color-mix(in srgb, #000000, 25% #FFFFFF)").map(|(_, f)| format(&f)).as_deref(), Some("color-mix(in srgb, #000000, #FFFFFF 25%)"));
        // Keywords need a base colour; a mix needs two colours.
        assert!(parse_line("oklch(l c h)").is_none() && parse_line("color-mix(in oklch, {A})").is_none());
        assert!(parse_line("oklch(from {A} calc(l - ) c h)").is_none());
        assert_eq!(number_formula_in("Spacing\ncalc({Space/Base} * 2 + 4)").map(|e| expr_text(&e, &keys(Model::Oklch))).as_deref(), Some("{Space/Base} * 2 + 4"));
        assert_eq!(without_number_formula("Spacing\ncalc({Base} * 2)"), "Spacing");
    }

    #[test]
    fn derived_variables_follow_their_bases_in_every_mode() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Theme", "modes": ["Light", "Dark"]})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.create", &json!({"collection": c, "name": "Primary", "type": "color", "values": {"Light": "#3A81F6", "Dark": "#93C5FD"}})).unwrap();
        s.execute("variable.create", &json!({"collection": c, "name": "Surface", "type": "color", "value": "#FFFFFF"})).unwrap();
        s.execute("variable.create", &json!({"collection": c, "name": "Base", "type": "number", "value": 8})).unwrap();
        for (name, kind, f) in [
            ("Primary/Faded", "color", "oklch(from {Primary} l c h / 50%)"),
            ("Primary/Hover", "color", "oklch(from {Primary/Faded} calc(l - 0.1) c h)"),
            ("Tint", "color", "color-mix(in oklch, {Primary} 20%, {Surface})"),
            ("Space/L", "number", "calc({Space/M} * 2)"),
            ("Space/M", "number", "calc({Base} * 2)"),
            ("Loop", "number", "calc({Loop} + 1)"),
        ] {
            let value = if kind == "color" { json!("#000000") } else { json!(0) };
            let v = s.execute("variable.create", &json!({"collection": c, "name": name, "type": kind, "value": value})).unwrap()["id"].as_str().unwrap().to_string();
            s.execute("variable.update", &json!({"variable": v, "description": f})).unwrap();
        }
        let get = |s: &Session, name: &str, mode: &str| {
            let doc = s.doc().unwrap();
            let m = doc.variables.collection(&c).unwrap().find_mode(mode).unwrap().id.clone();
            doc.variables.get(name).unwrap().values.get(&m).cloned().unwrap()
        };
        let color = |v: VariableValue| match v {
            VariableValue::Color(c) => c,
            other => panic!("{other:?}"),
        };
        let prof = s.doc().unwrap().color_profile;
        let primary = color(get(&s, "Primary", "Dark"));
        let faded = color(get(&s, "Primary/Faded", "Dark"));
        assert!(same(faded.with_alpha(1.0), primary) && (faded.a - 0.5).abs() < 0.01, "{faded:?}");
        let hover = color(get(&s, "Primary/Hover", "Dark"));
        assert!((Oklch::from_color(hover, prof).l - (Oklch::from_color(primary, prof).l - 0.1)).abs() < 0.01, "a chain: from a derived one");
        assert!((hover.a - 0.5).abs() < 0.01, "the alpha of its base");
        let tint = Oklch::from_color(color(get(&s, "Tint", "Light")), prof);
        assert!(tint.l > Oklch::from_color(color(get(&s, "Primary", "Light")), prof).l, "mostly the light surface");
        assert_eq!((get(&s, "Space/M", "Light"), get(&s, "Space/L", "Dark")), (VariableValue::Number(16.0), VariableValue::Number(32.0)));
        assert_eq!(get(&s, "Loop", "Light"), VariableValue::Number(0.0), "a loop is left alone");
        // The base changes: everything derived follows, in one undo step.
        s.execute("variable.set", &json!({"variable": "Base", "mode": "Light", "value": 10})).unwrap();
        assert_eq!(get(&s, "Space/L", "Light"), VariableValue::Number(40.0));
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(get(&s, "Space/L", "Light"), VariableValue::Number(32.0));
        // Editing a calculated number by hand detaches it.
        s.execute("variable.set", &json!({"variable": "Space/M", "mode": "Light", "value": 20})).unwrap();
        let doc = s.doc().unwrap();
        assert!(number_formula_in(&doc.variables.get("Space/M").unwrap().description).is_none());
        assert_eq!(get(&s, "Space/L", "Light"), VariableValue::Number(40.0), "what reads it follows the new value");
    }

    #[test]
    fn a_layer_fill_derives_from_a_variable_and_a_mix_detaches_when_edited() {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Theme"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.create", &json!({"collection": c, "name": "Primary", "type": "color", "value": "#FF0000"})).unwrap();
        let r = id(s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
        s.execute("node.set", &json!({"ids": [r], "props": {"pluginData": {NODE_KEY: "fills/0 color-mix(in srgb, {Primary}, #0000FF)"}}})).unwrap();
        assert_eq!(fill(&s, r).to_rgba8()[..3], [128, 0, 128]);
        s.execute("variable.set", &json!({"variable": "Primary", "value": "#00FF00"})).unwrap();
        assert_eq!(fill(&s, r).to_rgba8()[..3], [0, 128, 128]);
        s.execute("fill.set", &json!({"ids": [r], "index": 0, "color": "#123456"})).unwrap();
        assert!(s.doc().unwrap().node(NodeId(r)).unwrap().plugin_data.get(NODE_KEY).is_none(), "edited by hand: no formula");
        assert_eq!(fill(&s, r).to_hex(), "123456");
    }
}
