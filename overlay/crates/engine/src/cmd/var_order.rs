//! FigCraft Plus: ordering variables, groups and collections, and sorting them.
//!
//! The Variables editor lists a collection's variables in the order the file keeps them
//! (`Variables::variables`), groups in the order their first variable comes, and collections in
//! the order of `Variables::collections`. These commands change those orders (one undo step each):
//!
//! - `variable.move {variables, before?, group?}`: move variables before another one (or to the end
//!   of their group / collection), optionally into another group of the same collection (they are
//!   renamed `group/leaf`).
//! - `variable.moveGroup {collection, group, before?, into?}`: move a whole group (and its
//!   subgroups) before another group, optionally under another parent group (renamed).
//! - `variable.moveCollection {collection, before?}`: reorder collections.
//! - `variable.sort {collection, group?, by: name|value, mode?, descending?}`: sort the variables of
//!   a collection or group, each group on its own: by name in natural order (xs, s, m, l, xl, 2xl…;
//!   1, 2, 10), or by value (numbers; colours light to dark; text; booleans).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use figcraft_doc::{Document, Variable, VariableValue};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::*;
use crate::{Result, Session, bad, spec};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "variable.move",
            "Move variables",
            "{variables [ids or names], before? (variable id or name: they go right before it; none: the end of their group, or of the collection), group? (\"space\" or \"\" for none: move them into that group of the same collection, renamed group/leaf)}",
            true,
            move_variables
        ),
        spec!(
            "variable.moveGroup",
            "Move variable group",
            "{collection, group (\"space\" or \"color/brand\"), before? (another group's path: right before it; none: the end of its parent), into? (a parent group path, \"\" for the top: the group moves under it, renamed)} → {group}",
            true,
            move_group
        ),
        spec!("variable.moveCollection", "Move collection", "{collection, before? (collection id or name; none: the end)}", true, move_collection),
        spec!(
            "variable.sort",
            "Sort variables",
            "{collection, group? (a group path: it and its subgroups; none: the whole collection), by: name|value, mode? (for value: name or id; default the first), descending?} — each group is sorted on its own; groups keep their order",
            true,
            sort
        ),
    ]
}

/// The group part of a name ("space/md" → "space"; "" when ungrouped).
pub fn group_of(name: &str) -> &str {
    name.rsplit_once('/').map(|(g, _)| g).unwrap_or("")
}

fn leaf(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn in_group(name: &str, g: &str) -> bool {
    g.is_empty() || name.starts_with(&format!("{g}/"))
}

fn collection_id(doc: &Document, cmd: &str, c: &str) -> Result<String> {
    doc.variables.collection(c).map(|x| x.id.clone()).ok_or_else(|| bad(cmd, "no such collection"))
}

fn index_of(doc: &Document, v: &str) -> Option<usize> {
    let vars = &doc.variables.variables;
    vars.iter().position(|x| x.id == v).or_else(|| vars.iter().position(|x| x.name == v))
}

/// `base`, or `base 2`, `base 3`… so no other variable of collection `coll` has it.
fn unique_name(doc: &Document, coll: &str, base: &str, skip: &[String]) -> String {
    let taken = |n: &str| doc.variables.variables.iter().any(|x| x.collection == coll && x.name == n && !skip.contains(&x.id));
    if !taken(base) {
        return base.to_string();
    }
    (2..10_000).map(|k| format!("{base} {k}")).find(|n| !taken(n)).unwrap_or_else(|| base.to_string())
}

/// Clean a group path ("" for none).
fn clean_group(g: &str) -> String {
    g.split('/').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("/")
}

fn move_variables(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "variable.move";
    let names: Vec<String> = match get(p, "variables") {
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
        Some(Value::String(one)) => vec![one.clone()],
        _ => return Err(bad(cmd, "`variables`: [ids]")),
    };
    if names.is_empty() {
        return Err(bad(cmd, "`variables`: [ids]"));
    }
    let doc = s.doc_mut()?;
    let mut idx: Vec<usize> = names.iter().map(|n| index_of(doc, n).ok_or_else(|| bad(cmd, format!("no variable `{n}`")))).collect::<Result<_>>()?;
    idx.sort_unstable();
    idx.dedup();
    let vars = &doc.variables.variables;
    let coll = vars.get(idx[0]).map(|v| v.collection.clone()).ok_or_else(|| bad(cmd, "no such variable"))?;
    if idx.iter().any(|i| vars.get(*i).is_none_or(|v| v.collection != coll)) {
        return Err(bad(cmd, "the variables must be in one collection"));
    }
    let ids: Vec<String> = idx.iter().filter_map(|i| vars.get(*i).map(|v| v.id.clone())).collect();
    let before = match string(p, "before") {
        Some(b) => {
            let bi = index_of(doc, &b).ok_or_else(|| bad(cmd, format!("no variable `{b}`")))?;
            let bv = doc.variables.variables.get(bi).ok_or_else(|| bad(cmd, "no such variable"))?;
            if bv.collection != coll {
                return Err(bad(cmd, "`before` must be in the same collection"));
            }
            if ids.contains(&bv.id) {
                return Err(bad(cmd, "`before` can't be one of the variables moved"));
            }
            Some(bv.id.clone())
        }
        None => None,
    };
    let group = string(p, "group").map(|g| clean_group(&g));
    // Rename into the group first (names must stay unique in the collection).
    if let Some(g) = &group {
        for id in &ids {
            let Some(v) = doc.variables.variables.iter().find(|x| x.id == *id) else { continue };
            let want = if g.is_empty() { leaf(&v.name).to_string() } else { format!("{g}/{}", leaf(&v.name)) };
            if want == v.name {
                continue;
            }
            if v.extends.is_some() {
                return Err(bad(cmd, "an inherited variable's name comes from the collection it extends"));
            }
            let name = unique_name(doc, &coll, &want, &[]);
            if let Some(v) = doc.variables.variables.iter_mut().find(|x| x.id == *id) {
                v.name = name;
            }
        }
    }
    // Take them out (in order) and put them back at the target.
    let mut moved: Vec<Variable> = Vec::new();
    let mut rest: Vec<Variable> = Vec::new();
    for v in std::mem::take(&mut doc.variables.variables) {
        if ids.contains(&v.id) { moved.push(v) } else { rest.push(v) }
    }
    let at = match &before {
        Some(b) => rest.iter().position(|v| v.id == *b).unwrap_or(rest.len()),
        None => {
            // The end of the target group (or of the collection).
            let g = group.clone().or_else(|| moved.first().map(|v| group_of(&v.name).to_string())).unwrap_or_default();
            let last_in = rest.iter().rposition(|v| v.collection == coll && group_of(&v.name) == g);
            let last_coll = rest.iter().rposition(|v| v.collection == coll);
            last_in.or(last_coll).map_or(rest.len(), |i| i + 1)
        }
    };
    let tail = rest.split_off(at.min(rest.len()));
    rest.extend(moved);
    rest.extend(tail);
    doc.variables.variables = rest;
    Ok(json!({"moved": ids}))
}

fn move_group(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "variable.moveGroup";
    let doc = s.doc_mut()?;
    let coll = collection_id(doc, cmd, &req_string(cmd, p, "collection")?)?;
    let g = clean_group(&req_string(cmd, p, "group")?);
    if g.is_empty() {
        return Err(bad(cmd, "`group` can't be empty"));
    }
    let members: Vec<String> =
        doc.variables.variables.iter().filter(|v| v.collection == coll && in_group(&v.name, &g)).map(|v| v.id.clone()).collect();
    if members.is_empty() {
        return Err(bad(cmd, format!("no group `{g}`")));
    }
    // A new parent: rename every member (the group keeps its last part).
    let mut group = g.clone();
    if let Some(into) = string(p, "into").map(|i| clean_group(&i)) {
        if into == g || into.starts_with(&format!("{g}/")) {
            return Err(bad(cmd, "a group can't move into itself"));
        }
        let new = if into.is_empty() { leaf(&g).to_string() } else { format!("{into}/{}", leaf(&g)) };
        if new != g {
            if doc.variables.variables.iter().any(|v| v.collection == coll && in_group(&v.name, &new)) {
                return Err(bad(cmd, format!("a group `{new}` already exists there")));
            }
            for id in &members {
                if let Some(v) = doc.variables.variables.iter_mut().find(|x| x.id == *id) {
                    if v.extends.is_some() {
                        return Err(bad(cmd, "an inherited variable's name comes from the collection it extends"));
                    }
                    v.name = format!("{new}{}", &v.name[g.len()..]);
                }
            }
            group = new;
        }
    }
    let before = string(p, "before").map(|b| clean_group(&b)).filter(|b| !b.is_empty());
    if before.as_deref().is_some_and(|b| b == group || b.starts_with(&format!("{group}/"))) {
        return Err(bad(cmd, "`before` can't be the group moved or inside it"));
    }
    let mut moved: Vec<Variable> = Vec::new();
    let mut rest: Vec<Variable> = Vec::new();
    for v in std::mem::take(&mut doc.variables.variables) {
        if members.contains(&v.id) { moved.push(v) } else { rest.push(v) }
    }
    let at = match &before {
        Some(b) => rest.iter().position(|v| v.collection == coll && in_group(&v.name, b)),
        None => None,
    }
    .unwrap_or_else(|| {
        // The end of its parent group, else of the collection.
        let parent = group_of(&group).to_string();
        let last_in = if parent.is_empty() { None } else { rest.iter().rposition(|v| v.collection == coll && in_group(&v.name, &parent)) };
        last_in.or_else(|| rest.iter().rposition(|v| v.collection == coll)).map_or(rest.len(), |i| i + 1)
    });
    let tail = rest.split_off(at.min(rest.len()));
    rest.extend(moved);
    rest.extend(tail);
    doc.variables.variables = rest;
    Ok(json!({"group": group}))
}

fn move_collection(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "variable.moveCollection";
    let doc = s.doc_mut()?;
    let c = collection_id(doc, cmd, &req_string(cmd, p, "collection")?)?;
    let before = match string(p, "before") {
        Some(b) => Some(collection_id(doc, cmd, &b)?),
        None => None,
    };
    if before.as_deref() == Some(c.as_str()) {
        return Ok(json!({}));
    }
    let list = &mut doc.variables.collections;
    let from = list.iter().position(|x| x.id == c).ok_or_else(|| bad(cmd, "no such collection"))?;
    let item = list.remove(from);
    let at = before.and_then(|b| list.iter().position(|x| x.id == b)).unwrap_or(list.len());
    list.insert(at, item);
    Ok(json!({}))
}

// ---- sorting ----------------------------------------------------------------------------------

/// One piece of a name for natural sorting.
#[derive(Clone, Debug, PartialEq)]
enum Part {
    /// A T-shirt size: 3xs < 2xs < xs < s < m < l < xl < 2xl < 3xl…
    Size(i32),
    Num(f64),
    Text(String),
}

/// A T-shirt size's rank (`m` is 0).
fn size_rank(s: &str) -> Option<i32> {
    let s = s.to_ascii_lowercase();
    match s.as_str() {
        "xxs" => return Some(-3),
        "xs" => return Some(-2),
        "s" | "sm" => return Some(-1),
        "m" | "md" => return Some(0),
        "l" | "lg" => return Some(1),
        "xl" => return Some(2),
        "xxl" => return Some(3),
        "xxxl" => return Some(4),
        _ => {}
    }
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    let n: i32 = digits.parse().ok().filter(|n: &i32| (1..=20).contains(n))?;
    match &s[digits.len()..] {
        "xl" => Some(n + 1),
        "xs" => Some(-n - 1),
        _ => None,
    }
}

/// A name's parts: segments split on `-`, `_`, `.`, spaces; each a size, else runs of digits and
/// letters.
fn name_key(name: &str) -> Vec<Part> {
    let mut out = Vec::new();
    for seg in name.split(['-', '_', ' ', '.']).filter(|s| !s.is_empty()) {
        if let Some(r) = size_rank(seg) {
            out.push(Part::Size(r));
            continue;
        }
        let mut cur = String::new();
        let mut digits = false;
        for ch in seg.chars() {
            let d = ch.is_ascii_digit();
            if !cur.is_empty() && d != digits {
                out.push(if digits { Part::Num(cur.parse().unwrap_or(0.0)) } else { Part::Text(cur.to_lowercase()) });
                cur.clear();
            }
            digits = d;
            cur.push(ch);
        }
        if !cur.is_empty() {
            out.push(if digits { Part::Num(cur.parse().unwrap_or(0.0)) } else { Part::Text(cur.to_lowercase()) });
        }
    }
    out
}

fn part_cmp(a: &Part, b: &Part) -> Ordering {
    let rank = |p: &Part| match p {
        Part::Size(_) => 0,
        Part::Num(_) => 1,
        Part::Text(_) => 2,
    };
    match (a, b) {
        (Part::Size(x), Part::Size(y)) => x.cmp(y),
        (Part::Num(x), Part::Num(y)) => x.total_cmp(y),
        (Part::Text(x), Part::Text(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    }
}

/// Natural order of two names (their last parts).
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (ka, kb) = (name_key(leaf(a)), name_key(leaf(b)));
    for (x, y) in ka.iter().zip(kb.iter()) {
        let o = part_cmp(x, y);
        if o != Ordering::Equal {
            return o;
        }
    }
    ka.len().cmp(&kb.len()).then_with(|| leaf(a).cmp(leaf(b)))
}

/// A sortable value: (kind rank, number, text).
fn value_key(doc: &Document, v: &Variable, mode: &str) -> (u8, f64, String) {
    let modes = BTreeMap::from([(v.collection.clone(), mode.to_string())]);
    match doc.variables.resolve(&v.id, &modes) {
        Some(VariableValue::Number(n)) if n.is_finite() => (0, n, String::new()),
        // Light to dark (a palette's 50 … 950), so its descending OKLCH lightness.
        Some(VariableValue::Color(c)) => (1, -figcraft_color::Oklch::from_color(c, doc.color_profile).l, String::new()),
        Some(VariableValue::String(t)) => (2, 0.0, t.to_lowercase()),
        Some(VariableValue::Boolean(b)) => (3, if b { 1.0 } else { 0.0 }, String::new()),
        _ => (4, 0.0, String::new()),
    }
}

fn sort(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "variable.sort";
    let by = req_string(cmd, p, "by")?.to_ascii_lowercase();
    if by != "name" && by != "value" {
        return Err(bad(cmd, "`by`: name or value"));
    }
    let desc = boolean(p, "descending").unwrap_or(false);
    let doc = s.doc_mut()?;
    let coll_id = collection_id(doc, cmd, &req_string(cmd, p, "collection")?)?;
    let coll = doc.variables.collection(&coll_id).cloned().ok_or_else(|| bad(cmd, "no such collection"))?;
    let mode = match string(p, "mode") {
        Some(m) => coll.find_mode(&m).map(|x| x.id.clone()).ok_or_else(|| bad(cmd, format!("no mode `{m}`")))?,
        None => coll.default_mode().cloned().ok_or_else(|| bad(cmd, "the collection has no modes"))?,
    };
    let g = string(p, "group").map(|g| clean_group(&g)).unwrap_or_default();
    // The positions of the variables sorted; each group is sorted within itself and keeps its
    // place (groups are kept apart by their first variable's position).
    let pos: Vec<usize> =
        doc.variables.variables.iter().enumerate().filter(|(_, v)| v.collection == coll_id && in_group(&v.name, &g)).map(|(i, _)| i).collect();
    if pos.is_empty() {
        return Err(bad(cmd, "nothing to sort"));
    }
    let subset: Vec<Variable> = pos.iter().filter_map(|i| doc.variables.variables.get(*i).cloned()).collect();
    // Group order: by first appearance.
    let mut order: Vec<String> = Vec::new();
    for v in &subset {
        let gr = group_of(&v.name).to_string();
        if !order.contains(&gr) {
            order.push(gr);
        }
    }
    let rank = |v: &Variable| order.iter().position(|x| x == group_of(&v.name)).unwrap_or(usize::MAX);
    let keys: Vec<(usize, (u8, f64, String))> = subset.iter().map(|v| (rank(v), value_key(doc, v, &mode))).collect();
    let mut idx: Vec<usize> = (0..subset.len()).collect();
    idx.sort_by(|&a, &b| {
        let (ga, ka) = &keys[a];
        let (gb, kb) = &keys[b];
        ga.cmp(gb).then_with(|| {
            let o = if by == "name" {
                natural_cmp(&subset[a].name, &subset[b].name)
            } else {
                ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then_with(|| ka.2.cmp(&kb.2)).then_with(|| natural_cmp(&subset[a].name, &subset[b].name))
            };
            if desc { o.reverse() } else { o }
        })
    });
    let sorted: Vec<Variable> = idx.iter().filter_map(|i| subset.get(*i).cloned()).collect();
    for (slot, v) in pos.iter().zip(sorted) {
        if let Some(x) = doc.variables.variables.get_mut(*slot) {
            *x = v;
        }
    }
    Ok(json!({"sorted": pos.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(s: &Session, coll: &str) -> Vec<String> {
        s.doc().unwrap().variables.variables.iter().filter(|v| v.collection == coll).map(|v| v.name.clone()).collect()
    }

    fn setup() -> (Session, String) {
        let mut s = Session::new();
        let c = s.execute("variable.createCollection", &json!({"name": "Size", "modes": ["Desktop", "Mobile"]})).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        for (n, v) in [
            ("number/n-xs", 2),
            ("number/n-6xl", 96),
            ("number/n-s", 4),
            ("number/n-m", 8),
            ("number/n-l", 12),
            ("number/n-xl", 16),
            ("number/n-2xl", 24),
            ("number/n-3xl", 32),
            ("number/n-4xl", 48),
            ("number/n-5xl", 72),
        ] {
            s.execute("variable.create", &json!({"collection": c, "name": n, "type": "number", "value": v})).unwrap();
        }
        s.execute("variable.create", &json!({"collection": c, "name": "space/gap", "type": "number", "value": 10})).unwrap();
        (s, c)
    }

    #[test]
    fn natural_names_order_sizes_and_numbers() {
        let mut v = vec!["n-6xl", "n-xs", "n-2xl", "n-m", "n-s", "n-xl", "n-l", "n-3xs", "n-2xs"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["n-3xs", "n-2xs", "n-xs", "n-s", "n-m", "n-l", "n-xl", "n-2xl", "n-6xl"]);
        let mut v = vec!["item10", "item2", "item1", "item-3"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["item1", "item2", "item-3", "item10"]);
        let mut v = vec!["blue/950", "blue/50", "blue/100", "blue/500"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["blue/50", "blue/100", "blue/500", "blue/950"]);
    }

    #[test]
    fn sorting_by_name_or_value_fixes_an_import_and_undoes() {
        let (mut s, c) = setup();
        s.execute("variable.sort", &json!({"collection": c, "group": "number", "by": "value"})).unwrap();
        let want: Vec<String> = ["n-xs", "n-s", "n-m", "n-l", "n-xl", "n-2xl", "n-3xl", "n-4xl", "n-5xl", "n-6xl"]
            .iter()
            .map(|n| format!("number/{n}"))
            .chain(["space/gap".to_string()])
            .collect();
        assert_eq!(names(&s, &c), want);
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(names(&s, &c)[1], "number/n-6xl");
        s.execute("variable.sort", &json!({"collection": c, "by": "name"})).unwrap();
        assert_eq!(names(&s, &c), want, "by name gives the same order; space stays after number");
        s.execute("variable.sort", &json!({"collection": c, "group": "number", "by": "value", "descending": true})).unwrap();
        assert_eq!(names(&s, &c)[0], "number/n-6xl");
        assert!(s.execute("variable.sort", &json!({"collection": c, "by": "size"})).is_err());
    }

    #[test]
    fn variables_move_within_and_across_groups() {
        let (mut s, c) = setup();
        // n-6xl to the end of its group (after n-5xl).
        s.execute("variable.move", &json!({"variables": ["number/n-6xl"]})).unwrap();
        assert_eq!(names(&s, &c)[9], "number/n-6xl");
        // Two at once, before n-xs.
        s.execute("variable.move", &json!({"variables": ["number/n-m", "number/n-s"], "before": "number/n-xs"})).unwrap();
        assert_eq!(&names(&s, &c)[..3], ["number/n-s", "number/n-m", "number/n-xs"]);
        // Into another group: renamed, at its end.
        s.execute("variable.move", &json!({"variables": ["number/n-xs"], "group": "space"})).unwrap();
        let n = names(&s, &c);
        assert_eq!(n.last().map(String::as_str), Some("space/n-xs"));
        // A name taken in the target group gets a number.
        s.execute("variable.create", &json!({"collection": c, "name": "number/n-xs", "type": "number", "value": 1})).unwrap();
        s.execute("variable.move", &json!({"variables": ["number/n-xs"], "group": "space", "before": "space/gap"})).unwrap();
        assert!(names(&s, &c).contains(&"space/n-xs 2".to_string()));
        assert!(s.execute("variable.move", &json!({"variables": ["space/gap"], "before": "space/gap"})).is_err());
        assert!(s.execute("variable.move", &json!({"variables": []})).is_err());
    }

    #[test]
    fn groups_and_collections_move() {
        let (mut s, c) = setup();
        let r = s.execute("variable.moveGroup", &json!({"collection": c, "group": "space", "before": "number"})).unwrap();
        assert_eq!(r["group"], "space");
        assert_eq!(names(&s, &c)[0], "space/gap");
        // Under another group: renamed.
        let r = s.execute("variable.moveGroup", &json!({"collection": c, "group": "space", "into": "number"})).unwrap();
        assert_eq!(r["group"], "number/space");
        assert_eq!(names(&s, &c).last().map(String::as_str), Some("number/space/gap"));
        assert!(s.execute("variable.moveGroup", &json!({"collection": c, "group": "number", "into": "number/space"})).is_err());
        let d = s.execute("variable.createCollection", &json!({"name": "Palette"})).unwrap()["id"].as_str().unwrap().to_string();
        s.execute("variable.moveCollection", &json!({"collection": d, "before": c})).unwrap();
        assert_eq!(s.doc().unwrap().variables.collections[0].id, d);
        s.execute("variable.moveCollection", &json!({"collection": d})).unwrap();
        assert_eq!(s.doc().unwrap().variables.collections.last().unwrap().id, d);
    }
}
