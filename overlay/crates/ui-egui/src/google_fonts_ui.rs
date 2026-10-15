//! FigCraft Plus: Google Fonts and a font list without duplicates.
//!
//! - The Google Fonts catalog (every family and its styles) is read from fonts.google.com and kept
//!   in `google-fonts/catalog.json` next to the app (refreshed weekly; offline, the kept copy).
//! - Picking a family that isn't on this computer downloads its styles (static TrueType files)
//!   into `google-fonts/<Family>/` next to the app, so a portable copy carries them, and adds them.
//!   Families an open file uses that are missing here are fetched the same way.
//! - The font list has sections — Recent, In this file, Installed, Google Fonts — and lists every
//!   family once: a family on this computer (system, bundled or downloaded) is never offered again
//!   under Google Fonts.
//!
//! The desktop app supplies the network (an HTTPS round trip, [`set_transport`]); without it (the
//! web build, tests) the list shows the installed fonts only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use serde_json::Value;

use crate::FigApp;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets;

/// One HTTPS round trip: send `request` to `host` (port 443) and return the raw response (status
/// line, headers and body) once the server closes the connection.
pub type Transport = fn(host: &str, request: &[u8]) -> Result<Vec<u8>, String>;

static TRANSPORT: OnceLock<Transport> = OnceLock::new();

/// Let the font list reach Google Fonts through `t` (the desktop app calls this at start).
pub fn set_transport(t: Transport) {
    let _ = TRANSPORT.set(t);
}

const CATALOG_URL: &str = "https://fonts.google.com/metadata/fonts";
/// The kept catalog is refreshed after this long.
const CATALOG_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);
/// Responses larger than this are refused (the catalog is a few MB, a font file well under one).
const MAX_RESPONSE: usize = 64 << 20;

// ---------------------------------------------------------------------------------------------
// HTTP

/// `https://host/path` → (host, path).
fn split_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    (!host.is_empty()).then(|| (host.to_string(), path.to_string()))
}

/// A response: status, headers (lower-case names) and body.
type Response = (u16, BTreeMap<String, String>, Vec<u8>);

/// A response: status, headers (lower-case names) and body (chunked transfer decoded).
fn parse_response(raw: &[u8]) -> Result<Response, String> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("the server's answer was cut off")?;
    let head = String::from_utf8_lossy(&raw[..end]);
    let mut lines = head.split("\r\n");
    let status = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|s| s.parse().ok()).ok_or("not an HTTP answer")?;
    let headers: BTreeMap<String, String> =
        lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    let body = &raw[end + 4..];
    let body = if headers.get("transfer-encoding").is_some_and(|v| v.to_ascii_lowercase().contains("chunked")) {
        dechunk(body)?
    } else {
        match headers.get("content-length").and_then(|v| v.parse::<usize>().ok()) {
            Some(n) if n <= body.len() => body[..n].to_vec(),
            Some(_) => return Err("the download was cut off".into()),
            None => body.to_vec(),
        }
    };
    Ok((status, headers, body))
}

fn dechunk(mut b: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = b.windows(2).position(|w| w == b"\r\n").ok_or("the download was cut off")?;
        let size_text = String::from_utf8_lossy(&b[..eol]);
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16).map_err(|_| "a malformed download")?;
        b = &b[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if b.len() < size || out.len() + size > MAX_RESPONSE {
            return Err("the download was cut off".into());
        }
        out.extend_from_slice(&b[..size]);
        b = b.get(size + 2..).unwrap_or(&[]);
    }
}

/// GET `url` (following up to five redirects); the body of a 200 answer.
fn get(url: &str) -> Result<Vec<u8>, String> {
    let transport = TRANSPORT.get().ok_or("no network in this build")?;
    let mut url = url.to_string();
    for _ in 0..6 {
        let (host, path) = split_url(&url).ok_or_else(|| format!("not an https address: {url}"))?;
        // A plain user agent gets TrueType files from the Google Fonts CSS (browsers get WOFF2).
        let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: FigCraft\r\nAccept: */*\r\nConnection: close\r\n\r\n");
        let raw = transport(&host, req.as_bytes())?;
        if raw.len() > MAX_RESPONSE {
            return Err("the download is too large".into());
        }
        let (status, headers, body) = parse_response(&raw)?;
        match status {
            200 => return Ok(body),
            301 | 302 | 303 | 307 | 308 => {
                let to = headers.get("location").ok_or("a redirect without an address")?;
                url = if to.starts_with("https://") { to.clone() } else { format!("https://{host}{to}") };
            }
            s => return Err(format!("{host} answered {s}")),
        }
    }
    Err("too many redirects".into())
}

// ---------------------------------------------------------------------------------------------
// The catalog and font files

/// A Google Fonts family: its name and styles ("400", "700i").
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Family {
    pub name: String,
    pub variants: Vec<String>,
}

/// The families in the fonts.google.com metadata (which may start with a `)]}'` guard).
fn parse_catalog(bytes: &[u8]) -> Result<Vec<Family>, String> {
    let start = bytes.iter().position(|b| *b == b'{').ok_or("not a font catalog")?;
    let v: Value = serde_json::from_slice(&bytes[start..]).map_err(|e| format!("not a font catalog ({e})"))?;
    let list = v.get("familyMetadataList").and_then(Value::as_array).ok_or("not a font catalog")?;
    let mut out: Vec<Family> = list
        .iter()
        .filter_map(|f| {
            let name = f.get("family")?.as_str()?.to_string();
            let mut variants: Vec<String> = f.get("fonts")?.as_object()?.keys().cloned().collect();
            variants.sort_by_key(|v| (v.ends_with('i'), v.trim_end_matches('i').parse::<u16>().unwrap_or(400)));
            (!variants.is_empty()).then_some(Family { name, variants })
        })
        .collect();
    out.sort_by_key(|f| f.name.to_lowercase());
    Ok(out)
}

/// A variant's style name, as fonts name them: "400" Regular, "700i" Bold Italic, "400i" Italic.
pub(crate) fn style_name(variant: &str) -> String {
    let italic = variant.ends_with('i');
    let w = variant.trim_end_matches('i').parse::<u16>().unwrap_or(400);
    let weight = match w {
        0..=149 => "Thin",
        150..=249 => "ExtraLight",
        250..=349 => "Light",
        350..=449 => "Regular",
        450..=549 => "Medium",
        550..=649 => "SemiBold",
        650..=749 => "Bold",
        750..=849 => "ExtraBold",
        _ => "Black",
    };
    match (weight, italic) {
        ("Regular", true) => "Italic".into(),
        (w, true) => format!("{w} Italic"),
        (w, false) => w.into(),
    }
}

/// The Google Fonts CSS listing every style of `f` (each `src` a TrueType file).
fn css_url(f: &Family) -> String {
    let name: String = f.name.chars().map(|c| if c == ' ' { '+' } else { c }).filter(|c| c.is_ascii_alphanumeric() || *c == '+').collect();
    format!("https://fonts.googleapis.com/css?family={name}:{}", f.variants.join(","))
}

/// The (variant, file address) pairs of a Google Fonts CSS.
fn parse_css(css: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for block in css.split("@font-face").skip(1) {
        let field = |name: &str| -> Option<&str> {
            let at = block.find(name)? + name.len();
            let rest = block[at..].trim_start().strip_prefix(':')?;
            Some(rest[..rest.find(';').unwrap_or(rest.len())].trim())
        };
        let italic = field("font-style").is_some_and(|s| s == "italic");
        let weight = field("font-weight").and_then(|w| w.split_whitespace().next()?.parse::<u16>().ok()).unwrap_or(400);
        let Some(url) = block.find("url(").and_then(|i| {
            let rest = &block[i + 4..];
            Some(rest[..rest.find(')')?].trim_matches(|c| c == '\'' || c == '"').to_string())
        }) else {
            continue;
        };
        let variant = format!("{weight}{}", if italic { "i" } else { "" });
        if !out.iter().any(|(v, _)| *v == variant) {
            out.push((variant, url));
        }
    }
    out
}

/// The folder the catalog and downloaded families live in: `google-fonts` next to the app (a
/// portable copy carries them), else in the temporary folder.
fn dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let beside = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("google-fonts")));
        if let Some(d) = beside
            && std::fs::create_dir_all(&d).is_ok()
            && std::fs::write(d.join(".write-test"), b"").is_ok()
        {
            let _ = std::fs::remove_file(d.join(".write-test"));
            return d;
        }
        std::env::temp_dir().join("figcraft-google-fonts")
    })
    .clone()
}

/// A family's folder name (letters, digits, spaces and dashes of its name).
fn folder_name(family: &str) -> String {
    family.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-').collect::<String>().trim().to_string()
}

/// Add the downloaded family `name` in `folder` to the font database: each `<variant>.ttf` as
/// that style of `name`. True when a face was added.
fn install_folder(folder: &std::path::Path, name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(folder) else { return false };
    let mut faces = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("ttf") {
            continue;
        }
        let Some(variant) = p.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
        let Ok(bytes) = std::fs::read(&p) else { continue };
        let bytes = figcraft_text::fontdb::FontBytes::Shared(std::sync::Arc::new(bytes));
        if let Some(f) = figcraft_text::Face::from_bytes(bytes, 0, Some(name), Some(&style_name(&variant))) {
            faces.push(f);
        }
    }
    if faces.is_empty() {
        return false;
    }
    let mut db = figcraft_text::db().write().unwrap_or_else(PoisonError::into_inner);
    for f in faces {
        db.add(f);
    }
    drop(db);
    figcraft_text::clear_layout_cache();
    true
}

// ---------------------------------------------------------------------------------------------
// State

#[derive(Clone, Debug, PartialEq)]
enum Job {
    Downloading,
    Failed(String),
}

#[derive(Default)]
struct State {
    catalog: Vec<Family>,
    /// Lower-case names of the Google families downloaded here.
    installed: BTreeSet<String>,
    jobs: BTreeMap<String, Job>,
    /// Why the catalog couldn't be read (shown under Google Fonts).
    catalog_error: Option<String>,
    started: bool,
    ctx: Option<egui::Context>,
    /// The families the open file uses (for "In this file" and fetching missing ones).
    doc_families: Vec<String>,
    doc_key: Option<(u64, u64)>,
}

fn state() -> MutexGuard<'static, State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(Default::default).lock().unwrap_or_else(PoisonError::into_inner)
}

fn repaint() {
    if let Some(c) = state().ctx.clone() {
        c.request_repaint();
    }
}

fn spawn(f: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(f);
    #[cfg(target_arch = "wasm32")]
    drop(f);
}

/// Start up (once): add the families downloaded before, read the kept catalog and refresh it when
/// it is old. Each frame: note the open file's families and fetch the missing Google ones.
pub(crate) fn tick(app: &FigApp, ctx: &egui::Context) {
    let start = {
        let mut s = state();
        let first = !s.started;
        s.started = true;
        if first {
            s.ctx = Some(ctx.clone());
        }
        first
    };
    if start && TRANSPORT.get().is_some() {
        spawn(startup);
    }
    let Some(d) = app.session.docs.get(app.session.active) else { return };
    let key = (d.uid, d.revision);
    if state().doc_key == Some(key) {
        return;
    }
    let mut fams: Vec<String> = Vec::new();
    let mut note = |f: &str| {
        if !f.is_empty() && !fams.iter().any(|x| x.eq_ignore_ascii_case(f)) {
            fams.push(f.to_string());
        }
    };
    for n in d.doc.nodes() {
        if let Some(t) = n.kind.text() {
            note(&t.style.font_family);
            for r in &t.runs {
                note(&r.style.font_family);
            }
        }
    }
    for s in &d.doc.styles {
        if let figcraft_doc::StyleKind::Text { style } = &s.kind {
            note(&style.font_family);
        }
    }
    fams.sort_by_key(|f| f.to_lowercase());
    // Missing families are looked up only when there is a catalog to fetch them from.
    let missing: Vec<String> = if state().catalog.is_empty() {
        Vec::new()
    } else {
        let db = figcraft_text::db().read().unwrap_or_else(PoisonError::into_inner);
        fams.iter().filter(|f| !db.has_family(f)).cloned().collect()
    };
    let mut s = state();
    s.doc_key = Some(key);
    s.doc_families = fams;
    let fetch: Vec<String> = missing
        .into_iter()
        .filter(|f| {
            let k = f.to_lowercase();
            s.catalog.iter().any(|c| c.name.to_lowercase() == k) && !s.jobs.contains_key(&k)
        })
        .collect();
    drop(s);
    for f in fetch {
        download(&f);
    }
}

fn startup() {
    let root = dir();
    // Families downloaded before.
    if let Ok(entries) = std::fs::read_dir(&root) {
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let name = std::fs::read_to_string(p.join("family.txt")).ok().map(|s| s.trim().to_string());
            if let Some(name) = name.filter(|n| !n.is_empty())
                && install_folder(&p, &name)
            {
                state().installed.insert(name.to_lowercase());
            }
        }
    }
    repaint();
    // The catalog: the kept copy, refreshed when old or missing.
    let path = root.join("catalog.json");
    let kept = std::fs::read(&path).ok().and_then(|b| parse_catalog(&b).ok());
    let fresh = std::fs::metadata(&path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age < CATALOG_MAX_AGE);
    if let Some(c) = kept.clone() {
        state().catalog = c;
        repaint();
    }
    if kept.is_some() && fresh {
        return;
    }
    match get(CATALOG_URL).and_then(|b| parse_catalog(&b).map(|c| (b, c))) {
        Ok((bytes, c)) => {
            let _ = std::fs::write(&path, bytes);
            let mut s = state();
            s.catalog = c;
            s.catalog_error = None;
            s.doc_key = None;
        }
        Err(e) if kept.is_none() => state().catalog_error = Some(e),
        Err(_) => {}
    }
    repaint();
}

/// Download the Google family `name` (all its styles) in the background and add it.
pub(crate) fn download(name: &str) {
    let key = name.to_lowercase();
    let fam = {
        let mut s = state();
        if matches!(s.jobs.get(&key), Some(Job::Downloading)) || s.installed.contains(&key) {
            return;
        }
        let Some(f) = s.catalog.iter().find(|f| f.name.to_lowercase() == key).cloned() else { return };
        s.jobs.insert(key.clone(), Job::Downloading);
        f
    };
    spawn(move || {
        let result = fetch_family(&fam);
        let mut s = state();
        match result {
            Ok(()) => {
                s.installed.insert(key.clone());
                s.jobs.remove(&key);
            }
            Err(e) => {
                s.jobs.insert(key, Job::Failed(e));
            }
        }
        drop(s);
        repaint();
    });
}

fn fetch_family(f: &Family) -> Result<(), String> {
    let css = String::from_utf8_lossy(&get(&css_url(f))?).to_string();
    let files = parse_css(&css);
    if files.is_empty() {
        return Err("Google Fonts listed no files".into());
    }
    let folder = dir().join(folder_name(&f.name));
    std::fs::create_dir_all(&folder).map_err(|e| format!("can't save fonts in {} ({e})", folder.display()))?;
    for (variant, url) in files {
        let bytes = get(&url)?;
        std::fs::write(folder.join(format!("{variant}.ttf")), bytes).map_err(|e| e.to_string())?;
    }
    std::fs::write(folder.join("family.txt"), &f.name).map_err(|e| e.to_string())?;
    if install_folder(&folder, &f.name) { Ok(()) } else { Err("the downloaded files aren't fonts".into()) }
}

// ---------------------------------------------------------------------------------------------
// The font list

const ROW: f32 = 26.0;
const RECENT_KEY: &str = "figcraft_recent_fonts";

enum Row {
    Header(String),
    /// A note and its details (shown on hover).
    Note(String, String),
    /// A family on this computer; true when it came from Google Fonts.
    Installed(String, bool),
    /// A Google family not downloaded yet.
    Google(String),
}

/// The rows of the list for query `q` (lower case): Recent and In this file (without a query),
/// Installed, then the Google families not on this computer. Each family once per section, and a
/// family is either installed or offered by Google, never both.
fn rows(q: &str, installed: &[String], recent: &[String], in_file: &[String], s: &State) -> Vec<Row> {
    let hit = |f: &str| q.is_empty() || f.to_lowercase().contains(q);
    let local: BTreeSet<String> = installed.iter().map(|f| f.to_lowercase()).collect();
    let from_google = |f: &str| s.installed.contains(&f.to_lowercase());
    let mut out = Vec::new();
    if q.is_empty() {
        let recent: Vec<&String> = recent.iter().filter(|f| local.contains(&f.to_lowercase())).collect();
        if !recent.is_empty() {
            out.push(Row::Header(tr("Recent").into()));
            out.extend(recent.into_iter().map(|f| Row::Installed(f.clone(), from_google(f))));
        }
        if !in_file.is_empty() {
            out.push(Row::Header(tr("In this file").into()));
            for f in in_file {
                if local.contains(&f.to_lowercase()) {
                    out.push(Row::Installed(f.clone(), from_google(f)));
                } else {
                    out.push(Row::Google(f.clone()));
                }
            }
        }
    }
    let mine: Vec<&String> = installed.iter().filter(|f| hit(f)).collect();
    if !mine.is_empty() {
        out.push(Row::Header(tr("Installed").into()));
        out.extend(mine.into_iter().map(|f| Row::Installed(f.clone(), from_google(f))));
    }
    let google: Vec<&Family> = s.catalog.iter().filter(|f| hit(&f.name) && !local.contains(&f.name.to_lowercase())).collect();
    if !google.is_empty() || s.catalog_error.is_some() {
        out.push(Row::Header(tr("Google Fonts").into()));
    }
    out.extend(google.into_iter().map(|f| Row::Google(f.name.clone())));
    if let Some(e) = &s.catalog_error
        && s.catalog.is_empty()
    {
        out.push(Row::Note(tr("No connection to Google Fonts").into(), e.clone()));
    }
    out
}

/// The searchable, sectioned family list (replaces FigCraft's flat one). Returns the family
/// picked; a Google family is downloaded first and returned once it is installed (the list stays
/// open meanwhile).
pub(crate) fn font_picker(ui: &mut Ui, current: Option<&str>, just_opened: bool) -> Option<String> {
    let t = Tokens::get(ui.ctx());
    let q_id = Id::new("font_query");
    let wait_id = Id::new("font_pick_after_download");
    let mut q: String = ui.data(|d| d.get_temp(q_id)).unwrap_or_default();
    if just_opened {
        q.clear();
        ui.data_mut(|d| d.remove::<String>(wait_id));
    }
    let search = ui.add(egui::TextEdit::singleline(&mut q).hint_text(tr("Search fonts")).desired_width(ui.available_width()).font(regular(11.5)));
    if just_opened {
        search.request_focus();
    }
    ui.data_mut(|d| d.insert_temp(q_id, q.clone()));
    let ql = q.trim().to_lowercase();
    let installed = figcraft_text::family_names();
    let is_local = |f: &str| installed.iter().any(|a| a.eq_ignore_ascii_case(f));
    // A Google family picked earlier: hand it over once it is here.
    if let Some(w) = ui.data(|d| d.get_temp::<String>(wait_id)) {
        if is_local(&w) {
            ui.data_mut(|d| d.remove::<String>(wait_id));
            return Some(w);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    let recent: Vec<String> = ui.ctx().data_mut(|d| d.get_persisted::<Vec<String>>(Id::new(RECENT_KEY))).unwrap_or_default();
    let (list, jobs) = {
        let s = state();
        let in_file = s.doc_families.clone();
        (rows(&ql, &installed, &recent, &in_file, &s), s.jobs.clone())
    };
    let first = list.iter().find_map(|r| match r {
        Row::Installed(f, _) | Row::Google(f) => Some(f.clone()),
        _ => None,
    });
    let mut picked = None;
    let mut fetch = None;
    if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        match first {
            Some(f) if is_local(&f) => return Some(f),
            Some(f) => fetch = Some(f),
            None => {}
        }
    }
    if list.iter().all(|r| matches!(r, Row::Header(_) | Row::Note(..))) && list.iter().all(|r| !matches!(r, Row::Note(..))) {
        widgets::label(ui, tr("No fonts match"), t.text_tertiary);
        return None;
    }
    let waiting: Option<String> = ui.data(|d| d.get_temp(wait_id));
    let mut budget = 4;
    egui::ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show_rows(ui, ROW, list.len(), |ui, range| {
        for r in list.get(range).unwrap_or(&[]) {
            let w = ui.available_width();
            let clickable = matches!(r, Row::Installed(..) | Row::Google(_));
            let (rect, resp) = ui.allocate_exact_size(vec2(w, ROW), if clickable { Sense::click() } else { Sense::hover() });
            match r {
                Row::Header(h) => {
                    ui.painter().text(pos2(rect.min.x + 8.0, rect.center().y), Align2::LEFT_CENTER, h, semibold(10.5), t.text_secondary);
                }
                Row::Note(n, detail) => {
                    ui.painter().text(pos2(rect.min.x + 8.0, rect.center().y), Align2::LEFT_CENTER, n, regular(10.5), t.text_tertiary);
                    resp.on_hover_text(detail.as_str());
                }
                Row::Installed(f, _) | Row::Google(f) => {
                    let google_row = matches!(r, Row::Google(_));
                    let from_google = matches!(r, Row::Installed(_, true));
                    let hovered = resp.hovered();
                    if hovered {
                        ui.painter().rect_filled(rect, CornerRadius::same(5), t.accent);
                    }
                    let col = if hovered { Color32::WHITE } else { t.text };
                    let dim = if hovered { Color32::from_white_alpha(200) } else { t.text_tertiary };
                    if current.is_some_and(|c| c.eq_ignore_ascii_case(f)) {
                        icons::paint(
                            ui.painter(),
                            Rect::from_center_size(pos2(rect.min.x + 10.0, rect.center().y), vec2(12.0, 12.0)),
                            Icon::Check,
                            col,
                        );
                    }
                    // The source, at the right end.
                    let job = jobs.get(&f.to_lowercase());
                    let tag = match (google_row, job) {
                        (_, Some(Job::Downloading)) => Some(tr("Downloading…").to_string()),
                        (true, Some(Job::Failed(_))) => Some(tr("Download failed").to_string()),
                        (true, None) => Some(tr("Download").to_string()),
                        (false, _) if from_google => Some("Google".to_string()),
                        _ => None,
                    };
                    let tag_w = tag.as_ref().map_or(0.0, |s| ui.painter().layout_no_wrap(s.clone(), regular(10.0), dim).size().x + 10.0);
                    if let Some(s) = &tag {
                        ui.painter().text(pos2(rect.max.x - 6.0, rect.center().y), Align2::RIGHT_CENTER, s, regular(10.0), dim);
                    }
                    let text_min = pos2(rect.min.x + 22.0, rect.min.y);
                    let max_w = rect.max.x - text_min.x - 6.0 - tag_w;
                    let tex = if google_row { None } else { crate::font_preview::family(ui.ctx(), f, 13.0, &mut budget) };
                    match tex {
                        Some(tex) => {
                            crate::font_preview::paint(ui.painter(), &tex, pos2(text_min.x, rect.center().y), max_w, ROW, col);
                        }
                        None => {
                            ui.painter().with_clip_rect(Rect::from_min_size(text_min, vec2(max_w.max(0.0), ROW))).text(
                                pos2(text_min.x, rect.center().y),
                                Align2::LEFT_CENTER,
                                f,
                                regular(12.0),
                                col,
                            );
                        }
                    }
                    let resp = match job {
                        Some(Job::Failed(e)) => resp.on_hover_text(e.as_str()),
                        _ => resp,
                    };
                    if resp.clicked() {
                        if google_row {
                            fetch = Some(f.clone());
                        } else {
                            picked = Some(f.clone());
                        }
                    }
                    if waiting.as_deref().is_some_and(|w| w.eq_ignore_ascii_case(f)) && !hovered {
                        ui.painter().rect_stroke(rect, CornerRadius::same(5), egui::Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
                    }
                }
            }
        }
    });
    if let Some(f) = fetch {
        download(&f);
        ui.data_mut(|d| d.insert_temp(wait_id, f));
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_http_answers_plain_chunked_and_redirects() {
        let (s, h, b) = parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nX: y\r\n\r\nhello!!").unwrap();
        assert_eq!((s, b.as_slice(), h.get("x").map(String::as_str)), (200, &b"hello"[..], Some("y")));
        let (_, _, b) = parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6;x=1\r\n world\r\n0\r\n\r\n").unwrap();
        assert_eq!(b, b"hello world");
        let (s, h, _) = parse_response(b"HTTP/1.1 302 Found\r\nLocation: /s/a.ttf\r\nContent-Length: 0\r\n\r\n").unwrap();
        assert_eq!((s, h.get("location").map(String::as_str)), (302, Some("/s/a.ttf")));
        assert!(parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nshort").is_err());
        assert!(parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel").is_err());
        assert!(parse_response(b"garbage").is_err());
        assert_eq!(split_url("https://fonts.gstatic.com/s/x.ttf"), Some(("fonts.gstatic.com".into(), "/s/x.ttf".into())));
        assert_eq!(split_url("https://a.b"), Some(("a.b".into(), "/".into())));
        assert_eq!(split_url("http://a.b/"), None);
    }

    #[test]
    fn reads_the_catalog_and_the_css() {
        let meta = br#")]}'
{"axisRegistry": [], "familyMetadataList": [
 {"family": "Roboto", "fonts": {"700i": {}, "400": {}, "100": {}, "400i": {}}},
 {"family": "ABeeZee", "fonts": {"400": {}, "400i": {}}},
 {"family": "Broken"}]}"#;
        let c = parse_catalog(meta).unwrap();
        assert_eq!(c.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["ABeeZee", "Roboto"]);
        assert_eq!(c[1].variants, ["100", "400", "400i", "700i"]);
        assert_eq!(css_url(&c[1]), "https://fonts.googleapis.com/css?family=Roboto:100,400,400i,700i");
        assert_eq!(
            css_url(&Family { name: "Open Sans".into(), variants: vec!["400".into()] }),
            "https://fonts.googleapis.com/css?family=Open+Sans:400"
        );
        let css = "@font-face {\n  font-family: 'Roboto';\n  font-style: italic;\n  font-weight: 700;\n  font-stretch: normal;\n  src: url(https://fonts.gstatic.com/s/roboto/v1/b.ttf) format('truetype');\n}\n@font-face {\n  font-family: 'Roboto';\n  font-style: normal;\n  font-weight: 400;\n  src: url(https://fonts.gstatic.com/s/roboto/v1/r.ttf) format('truetype');\n}\n";
        assert_eq!(
            parse_css(css),
            [
                ("700i".to_string(), "https://fonts.gstatic.com/s/roboto/v1/b.ttf".to_string()),
                ("400".into(), "https://fonts.gstatic.com/s/roboto/v1/r.ttf".into())
            ]
        );
        assert!(parse_catalog(b"<html>").is_err());
    }

    #[test]
    fn names_styles_like_figma() {
        let names: Vec<String> = ["100", "200", "300", "400", "400i", "500", "600i", "700", "800", "900i"].iter().map(|v| style_name(v)).collect();
        assert_eq!(names, ["Thin", "ExtraLight", "Light", "Regular", "Italic", "Medium", "SemiBold Italic", "Bold", "ExtraBold", "Black Italic"]);
        assert_eq!(folder_name("Open Sans"), "Open Sans");
        assert_eq!(folder_name("../x:y"), "xy");
    }

    #[test]
    fn lists_each_family_once() {
        let s = State {
            catalog: vec![
                Family { name: "Inter".into(), variants: vec!["400".into()] },
                Family { name: "Roboto".into(), variants: vec!["400".into()] },
                Family { name: "Lora".into(), variants: vec!["400".into()] },
            ],
            installed: ["roboto".to_string()].into(),
            ..State::default()
        };
        let installed = vec!["Arial".to_string(), "Inter".to_string(), "Roboto".to_string()];
        let label = |r: &Row| match r {
            Row::Header(h) => format!("# {h}"),
            Row::Note(n, _) => format!("! {n}"),
            Row::Installed(f, g) => format!("{f}{}", if *g { " (Google)" } else { "" }),
            Row::Google(f) => format!("{f} ↓"),
        };
        let all: Vec<String> = rows("", &installed, &[], &[], &s).iter().map(label).collect();
        // Inter is installed (system): listed once, not under Google Fonts.
        assert_eq!(all, ["# Installed", "Arial", "Inter", "Roboto (Google)", "# Google Fonts", "Lora ↓"]);
        let found: Vec<String> = rows("o", &installed, &["Roboto".into(), "Gone".into()], &["Lora".into()], &s).iter().map(label).collect();
        assert_eq!(found, ["# Installed", "Roboto (Google)", "# Google Fonts", "Lora ↓"]);
        let start: Vec<String> =
            rows("", &installed, &["Roboto".into(), "Gone".into()], &["Lora".into(), "Arial".into()], &s).iter().map(label).collect();
        assert_eq!(&start[..6], ["# Recent", "Roboto (Google)", "# In this file", "Lora ↓", "Arial", "# Installed"]);
    }
}
