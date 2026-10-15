//! FigCraft Plus: layer classes and HTML tags.
//!
//! A *class* is a named set of a layer's properties, by section — Layout, Appearance, Fill,
//! Stroke, Effects, Typography — taken from a layer (its template). Applying it gives a layer
//! those properties at once, with the variables and styles they use; the layer remembers its
//! class. One class a layer, for now.
//!
//! - A layer's value that differs from its class's is an *override*: it stays when the class
//!   changes. `class.reset` puts the class's values back.
//! - `class.update {class, from}` takes the class's sections from a layer and passes the change on
//!   to every layer of the class, except where they override it.
//! - Sections that mean nothing for a layer are skipped (Typography on a rectangle, Layout's auto
//!   layout on a text: a text still takes Fill/Hug sizing).
//!
//! A layer's *HTML tag* (`p`, `h1`…`h6`, `span` for texts; `div`, `section`, `header`… for frames)
//! says what it is in a page; classes will use it to style the texts inside a frame by tag.
//!
//! Stored in the file as plugin data, so FigCraft without Plus opens it unchanged: the classes on
//! the first page (`figcraftplus.classes`), a layer's class and tag on the layer
//! (`figcraftplus.class`, `figcraftplus.tag`). Layers always hold their own values.

use std::collections::BTreeMap;

use figcraft_doc::{Document, Node, NodeId, NodeKind, Sizing, TextAutoResize};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::*;
use crate::{Result, Session, bad, spec};

/// Plugin data keys.
pub const CLASSES_KEY: &str = "figcraftplus.classes";
pub const CLASS_KEY: &str = "figcraftplus.class";
pub const TAG_KEY: &str = "figcraftplus.tag";

/// The sections a class can hold, in panel order.
pub const SECTIONS: [&str; 6] = ["layout", "appearance", "fill", "stroke", "effects", "typography"];

/// The tags a text can have (`p` when none is set).
pub const TEXT_TAGS: [&str; 9] = ["p", "h1", "h2", "h3", "h4", "h5", "h6", "span", "label"];
/// The tags a frame or other layer can have (`div` when none is set).
pub const FRAME_TAGS: [&str; 10] = ["div", "section", "header", "nav", "footer", "main", "article", "button", "ul", "li"];

/// A class: its id, name, sections and the values it gives.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerClass {
    pub id: String,
    pub name: String,
    pub sections: Vec<String>,
    #[serde(default)]
    pub template: Template,
}

/// The values of a class's sections: fields (by key), variable bindings, styles.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Template {
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
    #[serde(default)]
    pub bound: BTreeMap<String, String>,
    #[serde(default)]
    pub styles: BTreeMap<String, String>,
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "class.create",
            "Create class",
            "{id (the layer it is taken from), name, sections? [layout|appearance|fill|stroke|effects|typography] (default: the ones the layer has)} → {class}; the layer gets the class",
            true,
            create
        ),
        spec!("class.apply", "Apply class", "{ids?, class (id or name)}", true, apply_cmd),
        spec!("class.reset", "Reset to class", "{ids?} (their class's values come back: overrides go)", true, reset),
        spec!("class.detach", "Detach class", "{ids?} (they keep their values)", true, detach),
        spec!(
            "class.update",
            "Update class",
            "{class, from (layer id), sections? (new section list)} (every layer of the class takes the change, except what it overrides)",
            true,
            update
        ),
        spec!("class.rename", "Rename class", "{class, name}", true, rename),
        spec!("class.delete", "Delete class", "{class} (its layers keep their values)", true, delete),
        spec!("class.list", "List classes", "{}", false, |s, _| {
            let doc = s.doc()?;
            let list: Vec<Value> = classes(doc)
                .iter()
                .map(|c| json!({"id": c.id, "name": c.name, "sections": c.sections, "layers": members(doc, &c.id).len()}))
                .collect();
            Ok(json!(list))
        }),
        spec!("layer.setTag", "Set HTML tag", "{ids?, tag (p, h1–h6, span, label for texts; div, section, header, nav, footer, main, article, button, ul, li; null: the default)}", true, set_tag),
    ]
}

// ---- storage --------------------------------------------------------------------------------

/// The file's classes.
pub fn classes(doc: &Document) -> Vec<LayerClass> {
    doc.pages
        .iter()
        .filter_map(|p| doc.node(*p)?.plugin_data.get(CLASSES_KEY))
        .find_map(|s| serde_json::from_str::<Vec<LayerClass>>(s).ok())
        .unwrap_or_default()
}

fn save(doc: &mut Document, list: &[LayerClass]) -> Result<()> {
    let pages = doc.pages.clone();
    let Some(first) = pages.first().copied() else { return Ok(()) };
    for p in pages.iter().skip(1) {
        if doc.node(*p).is_some_and(|n| n.plugin_data.contains_key(CLASSES_KEY)) {
            doc.get_mut(*p)?.plugin_data.remove(CLASSES_KEY);
        }
    }
    let text = serde_json::to_string(list).unwrap_or_default();
    let n = doc.get_mut(first)?;
    if list.is_empty() {
        n.plugin_data.remove(CLASSES_KEY);
    } else {
        n.plugin_data.insert(CLASSES_KEY.into(), text);
    }
    Ok(())
}

/// A class by id or name.
pub fn find<'a>(list: &'a [LayerClass], key: &str) -> Option<&'a LayerClass> {
    list.iter().find(|c| c.id == key).or_else(|| list.iter().find(|c| c.name == key))
}

/// The class of a layer (its id), if it has one.
pub fn class_of(n: &Node) -> Option<&str> {
    n.plugin_data.get(CLASS_KEY).map(String::as_str)
}

/// The layers of class `id`, on every page.
pub fn members(doc: &Document, id: &str) -> Vec<NodeId> {
    let mut out = Vec::new();
    for p in &doc.pages {
        for n in doc.subtree(*p) {
            if doc.node(n).and_then(class_of) == Some(id) {
                out.push(n);
            }
        }
    }
    out
}

/// A layer's HTML tag: the one set, or its default (`p` for texts, `div` for the rest).
pub fn tag_of(n: &Node) -> &str {
    match n.plugin_data.get(TAG_KEY) {
        Some(t) => t,
        None if matches!(n.kind, NodeKind::Text(_)) => "p",
        None => "div",
    }
}

// ---- fields ---------------------------------------------------------------------------------

/// Whether section `s` means something for layer `n`.
pub fn applies(n: &Node, s: &str) -> bool {
    match s {
        "typography" => matches!(n.kind, NodeKind::Text(_)),
        "layout" => n.kind.is_frame_like() || matches!(n.kind, NodeKind::Text(_)),
        _ => true,
    }
}

/// The field keys of section `s` for layer `n`.
fn fields(n: &Node, s: &str) -> &'static [&'static str] {
    let text = matches!(n.kind, NodeKind::Text(_));
    match s {
        "layout" if text => &["sizingH", "sizingV"],
        "layout" => &["layout", "clip", "sizingH", "sizingV"],
        "appearance" => &["opacity", "blend", "corners"],
        "fill" => &["fills"],
        "stroke" => &["strokes", "stroke"],
        "effects" => &["effects"],
        "typography" => &["text.style", "text.alignH", "text.alignV", "text.paragraphSpacing", "text.paragraphIndent", "text.listSpacing"],
        _ => &[],
    }
}

fn to<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn from<T: serde::de::DeserializeOwned>(v: &Value) -> Option<T> {
    serde_json::from_value(v.clone()).ok()
}

/// A field's value on a layer.
fn read(n: &Node, key: &str) -> Option<Value> {
    let t = match &n.kind {
        NodeKind::Text(t) => Some(t),
        _ => None,
    };
    Some(match key {
        "layout" => to(&n.layout),
        "clip" => to(&n.clip),
        "sizingH" => to(&n.child_layout.sizing_h),
        "sizingV" => to(&n.child_layout.sizing_v),
        "opacity" => to(&n.opacity),
        "blend" => to(&n.blend),
        "corners" => to(&n.corners),
        "fills" => to(&n.fills),
        "strokes" => to(&n.strokes),
        "stroke" => to(&n.stroke),
        "effects" => to(&n.effects),
        "text.style" => to(&t?.style),
        "text.alignH" => to(&t?.align_h),
        "text.alignV" => to(&t?.align_v),
        "text.paragraphSpacing" => to(&t?.paragraph_spacing),
        "text.paragraphIndent" => to(&t?.paragraph_indent),
        "text.listSpacing" => to(&t?.list_spacing),
        _ => return None,
    })
}

/// Set a field on layer `id` (what doesn't fit the layer is left alone).
fn write(doc: &mut Document, id: NodeId, key: &str, v: &Value) -> Result<()> {
    // Fill needs an auto layout parent; Hug needs auto layout or text.
    if let ("sizingH" | "sizingV", Some(s)) = (key, from::<Sizing>(v)) {
        let n = doc.get(id)?;
        let al_parent = n.parent.and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some());
        let can_hug = n.layout.is_some() || matches!(n.kind, NodeKind::Text(_));
        if (s == Sizing::Fill && !al_parent) || (s == Sizing::Hug && !can_hug) {
            return Ok(());
        }
    }
    let n = doc.get_mut(id)?;
    match key {
        "layout" => {
            if let Some(x) = from(v) {
                n.layout = x;
            }
        }
        "clip" => {
            if let Some(x) = from(v) {
                n.clip = x;
            }
        }
        "sizingH" | "sizingV" => {
            if let Some(x) = from(v) {
                if key == "sizingH" {
                    n.child_layout.sizing_h = x;
                } else {
                    n.child_layout.sizing_v = x;
                }
                let (h, w) = (n.child_layout.sizing_h, n.child_layout.sizing_v);
                if let NodeKind::Text(t) = &mut n.kind {
                    t.auto_resize = match (h, w) {
                        (Sizing::Hug, _) => TextAutoResize::WidthAndHeight,
                        (_, Sizing::Hug) => TextAutoResize::Height,
                        _ => TextAutoResize::None,
                    };
                }
            }
        }
        "opacity" => {
            if let Some(x) = from(v) {
                n.opacity = x;
            }
        }
        "blend" => {
            if let Some(x) = from(v) {
                n.blend = x;
            }
        }
        "corners" => {
            if let Some(x) = from(v) {
                n.corners = x;
            }
        }
        "fills" => {
            if let Some(x) = from(v) {
                n.fills = x;
            }
        }
        "strokes" => {
            if let Some(x) = from(v) {
                n.strokes = x;
            }
        }
        "stroke" => {
            if let Some(x) = from(v) {
                n.stroke = x;
            }
        }
        "effects" => {
            if let Some(x) = from(v) {
                n.effects = x;
            }
        }
        k if k.starts_with("text.") => {
            let NodeKind::Text(t) = &mut n.kind else { return Ok(()) };
            match k {
                "text.style" => {
                    if let Some(x) = from(v) {
                        t.style = x;
                        // The class styles the whole text.
                        t.runs.clear();
                    }
                }
                "text.alignH" => {
                    if let Some(x) = from(v) {
                        t.align_h = x;
                    }
                }
                "text.alignV" => {
                    if let Some(x) = from(v) {
                        t.align_v = x;
                    }
                }
                "text.paragraphSpacing" => {
                    if let Some(x) = from(v) {
                        t.paragraph_spacing = x;
                    }
                }
                "text.paragraphIndent" => {
                    if let Some(x) = from(v) {
                        t.paragraph_indent = x;
                    }
                }
                "text.listSpacing" => {
                    if let Some(x) = from(v) {
                        t.list_spacing = x;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

/// The section a variable binding belongs to.
fn bound_section(field: &str) -> Option<&'static str> {
    let head = field.split('/').next().unwrap_or(field);
    Some(match head {
        "fills" | "fill" => "fill",
        "strokes" | "stroke" => "stroke",
        f if f.starts_with("stroke") && f.ends_with("Weight") => "stroke",
        "effects" => "effects",
        "opacity" => "appearance",
        f if f.ends_with("Radius") => "appearance",
        "width" | "height" | "minWidth" | "maxWidth" | "minHeight" | "maxHeight" | "gap" | "crossGap" | "columnGap" | "rowGap" => "layout",
        f if f.starts_with("padding") => "layout",
        "fontSize" | "fontFamily" | "fontStyle" | "fontWeight" | "lineHeight" | "letterSpacing" | "paragraphSpacing" | "paragraphIndent" => {
            "typography"
        }
        _ => return None,
    })
}

/// The style slot of a section.
fn style_slot(s: &str) -> Option<&'static str> {
    Some(match s {
        "fill" => "fill",
        "stroke" => "stroke",
        "effects" => "effect",
        "typography" => "text",
        _ => return None,
    })
}

fn style_ref(n: &Node, slot: &str) -> Option<String> {
    match slot {
        "fill" => n.styles.fill.clone(),
        "stroke" => n.styles.stroke.clone(),
        "effect" => n.styles.effect.clone(),
        "text" => n.styles.text.clone(),
        _ => None,
    }
}

fn set_style_ref(n: &mut Node, slot: &str, v: Option<String>) {
    match slot {
        "fill" => n.styles.fill = v,
        "stroke" => n.styles.stroke = v,
        "effect" => n.styles.effect = v,
        "text" => n.styles.text = v,
        _ => {}
    }
}

/// The sections a layer has something in (for a new class).
pub fn default_sections(n: &Node) -> Vec<String> {
    SECTIONS
        .iter()
        .filter(|s| applies(n, s))
        .filter(|s| match **s {
            "fill" => !n.fills.is_empty(),
            "stroke" => !n.strokes.is_empty(),
            "effects" => !n.effects.is_empty(),
            "layout" => n.layout.is_some() || matches!(n.kind, NodeKind::Text(_)),
            _ => true,
        })
        .map(|s| s.to_string())
        .collect()
}

/// A layer's values for `sections`, as a template.
pub fn template_of(n: &Node, sections: &[String]) -> Template {
    let mut t = Template::default();
    for s in sections.iter().filter(|s| applies(n, s)) {
        for k in fields(n, s) {
            if let Some(v) = read(n, k) {
                t.values.insert((*k).to_string(), v);
            }
        }
        if let Some(slot) = style_slot(s)
            && let Some(id) = style_ref(n, slot)
        {
            t.styles.insert(slot.to_string(), id);
        }
    }
    for (f, v) in &n.bound {
        if bound_section(f).is_some_and(|s| sections.iter().any(|x| x == s)) {
            t.bound.insert(f.clone(), v.clone());
        }
    }
    t
}

/// Give layer `id` the values of `tpl` for `sections` (only the fields that differ from `old`,
/// when given: what the layer overrides stays).
fn give(doc: &mut Document, id: NodeId, sections: &[String], tpl: &Template, old: Option<&Template>) -> Result<()> {
    let Some(n) = doc.node(id).cloned() else { return Ok(()) };
    for s in sections.iter().filter(|s| applies(&n, s)) {
        for k in fields(&n, s) {
            let Some(v) = tpl.values.get(*k) else { continue };
            let cur = read(&n, k);
            if let Some(o) = old
                && o.values.get(*k) != cur.as_ref()
            {
                continue;
            }
            if cur.as_ref() != Some(v) {
                write(doc, id, k, v)?;
            }
        }
        if let Some(slot) = style_slot(s) {
            let cur = style_ref(&n, slot);
            let overridden = old.is_some_and(|o| o.styles.get(slot) != cur.as_ref());
            if !overridden {
                set_style_ref(doc.get_mut(id)?, slot, tpl.styles.get(slot).cloned());
            }
        }
        // Bindings of the section: the class's replace the layer's (unless it changed them).
        let fields_here: Vec<String> = n.bound.keys().chain(tpl.bound.keys()).filter(|f| bound_section(f) == Some(s.as_str())).cloned().collect();
        for f in fields_here {
            let cur = n.bound.get(&f).cloned();
            if old.is_some_and(|o| o.bound.get(&f) != cur.as_ref()) {
                continue;
            }
            let node = doc.get_mut(id)?;
            match tpl.bound.get(&f) {
                Some(v) => {
                    node.bound.insert(f, v.clone());
                }
                None => {
                    node.bound.remove(&f);
                }
            }
        }
    }
    Ok(())
}

/// The sections where layer `n` differs from its class (its overrides).
pub fn overrides(n: &Node, c: &LayerClass) -> Vec<String> {
    let mine = template_of(n, &c.sections);
    c.sections
        .iter()
        .filter(|s| applies(n, s))
        .filter(|s| {
            let keys = fields(n, s);
            let vals = keys.iter().any(|k| mine.values.get(*k) != c.template.values.get(*k));
            let slot = style_slot(s);
            let style = slot.is_some_and(|sl| mine.styles.get(sl) != c.template.styles.get(sl));
            let in_s = |m: &BTreeMap<String, String>| -> BTreeMap<String, String> {
                m.iter().filter(|(f, _)| bound_section(f) == Some(s.as_str())).map(|(a, b)| (a.clone(), b.clone())).collect()
            };
            vals || style || in_s(&mine.bound) != in_s(&c.template.bound)
        })
        .cloned()
        .collect()
}

// ---- commands -------------------------------------------------------------------------------

fn sections_param(cmd: &str, p: &Value) -> Result<Option<Vec<String>>> {
    let Some(v) = get(p, "sections") else { return Ok(None) };
    let list: Vec<String> = match v {
        Value::Array(a) => a.iter().filter_map(Value::as_str).map(str::to_lowercase).collect(),
        _ => return Err(bad(cmd, "`sections` must be a list")),
    };
    if let Some(x) = list.iter().find(|s| !SECTIONS.contains(&s.as_str())) {
        return Err(bad(cmd, format!("unknown section `{x}`")));
    }
    // In panel order, once each.
    Ok(Some(SECTIONS.iter().filter(|s| list.iter().any(|x| x == *s)).map(|s| s.to_string()).collect()))
}

fn class_param(cmd: &str, doc: &Document, p: &Value) -> Result<LayerClass> {
    let key = req_string(cmd, p, "class")?;
    find(&classes(doc), &key).cloned().ok_or_else(|| bad(cmd, format!("no class `{key}`")))
}

fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "class.create";
    let id = req_id(cmd, p, "id")?;
    let name = req_string(cmd, p, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(bad(cmd, "a class needs a name"));
    }
    let doc = s.doc_mut()?;
    let n = doc.get(id)?.clone();
    let sections = sections_param(cmd, p)?.unwrap_or_else(|| default_sections(&n));
    if sections.is_empty() {
        return Err(bad(cmd, "no sections"));
    }
    let mut list = classes(doc);
    let cid = (1..).map(|k| format!("C:{k}")).find(|c| list.iter().all(|x| &x.id != c)).unwrap_or_else(|| "C:1".into());
    list.push(LayerClass { id: cid.clone(), name, template: template_of(&n, &sections), sections });
    save(doc, &list)?;
    doc.get_mut(id)?.plugin_data.insert(CLASS_KEY.into(), cid.clone());
    Ok(json!({"class": cid}))
}

fn apply_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "class.apply";
    let ids = req_targets(cmd, s, p)?;
    let doc = s.doc_mut()?;
    let c = class_param(cmd, doc, p)?;
    for id in ids {
        give(doc, id, &c.sections, &c.template, None)?;
        doc.get_mut(id)?.plugin_data.insert(CLASS_KEY.into(), c.id.clone());
    }
    Ok(json!({"ok": true}))
}

fn reset(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = req_targets("class.reset", s, p)?;
    let doc = s.doc_mut()?;
    let list = classes(doc);
    for id in ids {
        let Some(c) = doc.node(id).and_then(class_of).and_then(|k| find(&list, k)).cloned() else { continue };
        give(doc, id, &c.sections, &c.template, None)?;
    }
    Ok(json!({"ok": true}))
}

fn detach(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = req_targets("class.detach", s, p)?;
    let doc = s.doc_mut()?;
    for id in ids {
        if doc.node(id).is_some_and(|n| n.plugin_data.contains_key(CLASS_KEY)) {
            doc.get_mut(id)?.plugin_data.remove(CLASS_KEY);
        }
    }
    Ok(json!({"ok": true}))
}

fn update(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "class.update";
    let from_id = req_id(cmd, p, "from")?;
    let doc = s.doc_mut()?;
    let old = class_param(cmd, doc, p)?;
    let src = doc.get(from_id)?.clone();
    let sections = sections_param(cmd, p)?.unwrap_or_else(|| old.sections.clone());
    // Sections the source layer can't give keep what the class had.
    let mut tpl = template_of(&src, &sections);
    for s in sections.iter().filter(|s| !applies(&src, s)) {
        for (k, v) in &old.template.values {
            if section_of_key(k) == Some(s.as_str()) {
                tpl.values.insert(k.clone(), v.clone());
            }
        }
    }
    for id in members(doc, &old.id) {
        if id == from_id {
            continue;
        }
        // Sections new to the class: given whole; the others: what the layer didn't change.
        let new_secs: Vec<String> = sections.iter().filter(|s| !old.sections.contains(s)).cloned().collect();
        let kept: Vec<String> = sections.iter().filter(|s| old.sections.contains(s)).cloned().collect();
        give(doc, id, &kept, &tpl, Some(&old.template))?;
        give(doc, id, &new_secs, &tpl, None)?;
    }
    let mut list = classes(doc);
    if let Some(c) = list.iter_mut().find(|c| c.id == old.id) {
        c.sections = sections;
        c.template = tpl;
    }
    save(doc, &list)?;
    doc.get_mut(from_id)?.plugin_data.insert(CLASS_KEY.into(), old.id);
    Ok(json!({"ok": true}))
}

/// The section of a template field key.
fn section_of_key(k: &str) -> Option<&'static str> {
    Some(match k {
        "layout" | "clip" | "sizingH" | "sizingV" => "layout",
        "opacity" | "blend" | "corners" => "appearance",
        "fills" => "fill",
        "strokes" | "stroke" => "stroke",
        "effects" => "effects",
        k if k.starts_with("text.") => "typography",
        _ => return None,
    })
}

fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "class.rename";
    let name = req_string(cmd, p, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(bad(cmd, "a class needs a name"));
    }
    let doc = s.doc_mut()?;
    let c = class_param(cmd, doc, p)?;
    let mut list = classes(doc);
    if let Some(x) = list.iter_mut().find(|x| x.id == c.id) {
        x.name = name;
    }
    save(doc, &list)?;
    Ok(json!({"ok": true}))
}

fn delete(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "class.delete";
    let doc = s.doc_mut()?;
    let c = class_param(cmd, doc, p)?;
    for id in members(doc, &c.id) {
        doc.get_mut(id)?.plugin_data.remove(CLASS_KEY);
    }
    let list: Vec<LayerClass> = classes(doc).into_iter().filter(|x| x.id != c.id).collect();
    save(doc, &list)?;
    Ok(json!({"ok": true}))
}

fn set_tag(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.setTag";
    let ids = req_targets(cmd, s, p)?;
    let tag = string(p, "tag").map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty());
    let doc = s.doc_mut()?;
    for id in ids {
        let n = doc.get(id)?;
        let text = matches!(n.kind, NodeKind::Text(_));
        match &tag {
            Some(t) => {
                let ok = if text { TEXT_TAGS.contains(&t.as_str()) } else { FRAME_TAGS.contains(&t.as_str()) };
                if !ok {
                    return Err(bad(cmd, format!("`{t}` isn't a tag for this layer")));
                }
                let default = if text { "p" } else { "div" };
                let n = doc.get_mut(id)?;
                if t == default {
                    n.plugin_data.remove(TAG_KEY);
                } else {
                    n.plugin_data.insert(TAG_KEY.into(), t.clone());
                }
            }
            None => {
                doc.get_mut(id)?.plugin_data.remove(TAG_KEY);
            }
        }
    }
    Ok(json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::id_of;

    fn ex(s: &mut Session, cmd: &str, p: Value) -> Value {
        s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd}: {e}"))
    }

    fn rect(s: &mut Session, x: f64) -> NodeId {
        id_of(&ex(s, "shape.rectangle", json!({"x": x, "y": 0, "width": 50, "height": 50}))["id"]).unwrap()
    }

    fn fill(s: &Session, id: NodeId) -> Value {
        to(&s.doc().unwrap().node(id).unwrap().fills)
    }

    #[test]
    fn a_class_gives_its_sections_and_follows_its_changes_except_overrides() {
        let mut s = Session::new();
        let (a, b, c) = (rect(&mut s, 0.0), rect(&mut s, 100.0), rect(&mut s, 200.0));
        ex(&mut s, "node.set", json!({"ids": [a], "props": {"fills": ["#ff0000"], "cornerRadius": 8, "opacity": 0.5}}));
        let cid = ex(&mut s, "class.create", json!({"id": a, "name": "Card", "sections": ["fill", "appearance"]}))["class"].clone();
        ex(&mut s, "class.apply", json!({"ids": [b, c], "class": "Card"}));
        assert_eq!(fill(&s, b), fill(&s, a));
        assert_eq!(s.doc().unwrap().node(c).unwrap().opacity, 0.5);
        // c overrides its fill; the class changes colour and radius.
        ex(&mut s, "node.set", json!({"ids": [c], "props": {"fills": ["#00ff00"]}}));
        let green = fill(&s, c);
        ex(&mut s, "node.set", json!({"ids": [a], "props": {"fills": ["#0000ff"], "cornerRadius": 16}}));
        ex(&mut s, "class.update", json!({"class": cid, "from": a}));
        assert_eq!(fill(&s, b), fill(&s, a), "b follows the class");
        assert_eq!(fill(&s, c), green, "c keeps its own fill");
        assert_eq!(to(&s.doc().unwrap().node(c).unwrap().corners), to(&s.doc().unwrap().node(a).unwrap().corners), "but takes the radius");
        let doc = s.doc().unwrap();
        let list = classes(doc);
        assert_eq!(overrides(doc.node(c).unwrap(), &list[0]), ["fill"]);
        assert!(overrides(doc.node(b).unwrap(), &list[0]).is_empty());
        ex(&mut s, "class.reset", json!({"ids": [c]}));
        assert_eq!(fill(&s, c), fill(&s, a), "reset brings the class back");
        assert_eq!(members(s.doc().unwrap(), "C:1").len(), 3);
        ex(&mut s, "class.detach", json!({"ids": [c]}));
        assert_eq!(members(s.doc().unwrap(), "C:1").len(), 2);
        ex(&mut s, "class.rename", json!({"class": "Card", "name": "card"}));
        ex(&mut s, "class.delete", json!({"class": "card"}));
        assert!(classes(s.doc().unwrap()).is_empty());
        assert_eq!(fill(&s, b), fill(&s, a), "layers keep their values");
    }

    #[test]
    fn typography_goes_to_texts_only_and_tags_are_checked() {
        let mut s = Session::new();
        let t1 = id_of(&ex(&mut s, "text.create", json!({"x": 0, "y": 0, "text": "Title"}))["id"]).unwrap();
        let t2 = id_of(&ex(&mut s, "text.create", json!({"x": 0, "y": 100, "text": "Other"}))["id"]).unwrap();
        let r = rect(&mut s, 300.0);
        ex(&mut s, "node.set", json!({"ids": [t1], "props": {"fontSize": 40, "fills": ["#123456"]}}));
        ex(&mut s, "class.create", json!({"id": t1, "name": "title", "sections": ["typography", "fill"]}));
        ex(&mut s, "class.apply", json!({"ids": [t2, r], "class": "title"}));
        let doc = s.doc().unwrap();
        let size = |id| match &doc.node(id).unwrap().kind {
            NodeKind::Text(t) => t.style.font_size,
            _ => 0.0,
        };
        assert_eq!(size(t2), 40.0);
        assert_eq!(fill(&s, r), fill(&s, t1), "a rectangle takes the fill, not the type");
        ex(&mut s, "layer.setTag", json!({"ids": [t1], "tag": "h1"}));
        assert_eq!(tag_of(s.doc().unwrap().node(t1).unwrap()), "h1");
        assert!(s.execute("layer.setTag", &json!({"ids": [r], "tag": "h1"})).is_err(), "a rectangle isn't a heading");
        ex(&mut s, "layer.setTag", json!({"ids": [r], "tag": "section"}));
        ex(&mut s, "layer.setTag", json!({"ids": [t1], "tag": null}));
        assert_eq!(tag_of(s.doc().unwrap().node(t1).unwrap()), "p");
    }
}
