//! Builds the Mati Mati website into `dist/` as plain static files, ready for
//! GitHub Pages:
//!
//! ```text
//! dist/index.html                      home (Spanish, the main language)
//! dist/revista/index.html              magazine: every article
//! dist/revista/<slug>/index.html       one article
//! dist/herramientas/, nosotros/, apoya/, contacto/
//! dist/feed.xml                        RSS of the articles (for the future newsletter)
//! dist/en/..., dist/zh/...             the same site in English and Chinese
//! dist/sitemap.xml, robots.txt, 404.html
//! dist/static/...                      css, js, images
//! ```
//!
//! Content lives outside the code:
//!   site.toml                    name, links (Ko-fi, Instagram, email), tools
//!   i18n/<locale>.toml           every interface text, per language
//!   content/articles/<locale>/   one Markdown file per article
//!   content/instagram.toml       Instagram posts shown on the home page
//!   templates/                   HTML (MiniJinja, Jinja-like syntax)
//!
//! Usage:
//!   cargo run                          build with base "/"
//!   cargo run -- --serve               build and serve on http://127.0.0.1:3000
//!   cargo run -- --base matimati --serve
//!       build as on GitHub Pages (site under /matimati/). Write the base
//!       without a leading slash: Git Bash on Windows rewrites "/x" into a
//!       Windows path.
//!   cargo run -- --drafts              also build articles with draft = true

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

mod instagram;

use minijinja::{context, path_loader, Environment, Value};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

struct Options {
    base: String,
    out: PathBuf,
    drafts: bool,
    serve: Option<u16>,
}

fn parse_args(root: &Path) -> Options {
    let mut opts = Options {
        base: "/".into(),
        out: root.join("dist"),
        drafts: false,
        serve: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                let mut base = args.next().expect("--base needs a value, e.g. matimati");
                if !base.starts_with('/') {
                    base.insert(0, '/');
                }
                if !base.ends_with('/') {
                    base.push('/');
                }
                opts.base = base;
            }
            "--out" => opts.out = PathBuf::from(args.next().expect("--out needs a directory")),
            "--drafts" => opts.drafts = true,
            "--serve" => opts.serve = opts.serve.or(Some(3000)),
            "--port" => {
                opts.serve = Some(
                    args.next()
                        .and_then(|p| p.parse().ok())
                        .expect("--port needs a number"),
                )
            }
            other => panic!("unknown argument: {other}"),
        }
    }
    opts
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let opts = parse_args(&root);
    match build(&root, &opts) {
        Ok(n) => println!("Site built in {} ({n} pages)", opts.out.display()),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
    if let Some(port) = opts.serve {
        serve(&opts.out, &opts.base, port);
    }
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

/// `site.toml`.
#[derive(Deserialize, Serialize)]
struct Config {
    name: String,
    /// Public address of the site, without a trailing slash. Used for
    /// absolute links (RSS, sitemap, social previews).
    url: String,
    #[serde(default)]
    email: String,
    instagram: String,
    kofi: String,
    #[serde(default)]
    newsletter: Newsletter,
    #[serde(default)]
    ads: Ads,
    /// Mati Mati projects: the home carousel, and the tools page (those
    /// with `tool = true`).
    #[serde(default)]
    projects: Vec<Project>,
}

/// Email sign-up form. Off until a newsletter service is chosen.
#[derive(Deserialize, Serialize, Default)]
struct Newsletter {
    #[serde(default)]
    enabled: bool,
    /// The service's form address (MailerLite, Buttondown…).
    #[serde(default)]
    action: String,
}

/// Advertising. Off until the site has its own domain.
#[derive(Deserialize, Serialize, Default)]
struct Ads {
    #[serde(default)]
    enabled: bool,
    /// Google AdSense publisher id, "ca-pub-…".
    #[serde(default)]
    client: String,
    /// Ad unit id for the slots inside pages.
    #[serde(default)]
    slot: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Project {
    id: String,
    /// External address (a tool on its own site)…
    #[serde(default)]
    url: Option<String>,
    /// …or a page of this site, by route id ("magazine", "support"…).
    #[serde(default)]
    page: Option<String>,
    /// Fixed colour series; empty = the palette of the day.
    #[serde(default)]
    serie: String,
    /// Illustration in the carousel: "phone", "magazine", "verbs", "heart".
    #[serde(default = "default_art")]
    art: String,
    /// Also listed on the tools page.
    #[serde(default)]
    tool: bool,
    /// Texts per locale ("es", "en", "zh").
    text: BTreeMap<String, ProjectText>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct ProjectText {
    name: String,
    description: String,
    #[serde(default)]
    tag: String,
    /// Text of the main button (default: "Open").
    #[serde(default)]
    button: String,
    /// Address of this locale's version, if it differs.
    #[serde(default)]
    url: Option<String>,
}

fn default_art() -> String {
    "phone".into()
}

/// The fields of `i18n/<locale>.toml` the generator itself needs. The whole
/// file also goes to the templates as `t`.
#[derive(Deserialize)]
struct LocaleMeta {
    html_lang: String,
    hreflang: String,
    og_locale: String,
    label: String,
    switch_name: String,
    months: Vec<String>,
    /// "{d}", "{m}" (number), "{month}" (name), "{y}".
    date_format: String,
    /// Words per minute for the reading time; Chinese counts characters.
    reading_rate: usize,
    routes: BTreeMap<String, String>,
}

struct Locale {
    code: &'static str,
    /// Folder of the locale inside the site: "", "en/", "zh/".
    prefix: String,
    meta: LocaleMeta,
    strings: toml::Value,
}

/// Spanish first: it is the main language and lives at the root.
const LOCALES: [&str; 3] = ["es", "en", "zh"];

/// Pages that exist in every locale, by route id (see `[routes]` in i18n).
const PAGES: [&str; 5] = ["magazine", "tools", "about", "support", "contact"];

/// Colour series of the manual (§4 C). The site shows one per day, in this
/// order (static/js/theme.js has the same list).
const SERIES: [&str; 8] = [
    "mati", "mar", "milpa", "jacaranda", "fiesta", "anil", "barro", "noche",
];

/// Instagram posts on the home page.
const INSTAGRAM_POSTS: usize = 6;


/// Front matter of an article, between `+++` lines, in TOML.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Front {
    title: String,
    description: String,
    /// "YYYY-MM-DD".
    date: String,
    /// Same key in every translation of an article, so the language switch
    /// can link them. Defaults to the file name.
    key: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// Colour series of the article; empty = the palette of the day.
    #[serde(default)]
    serie: String,
    author: Option<String>,
    /// Optional cover image, relative to static/ (e.g. "img/articulos/x.jpg").
    image: Option<String>,
    #[serde(default)]
    image_alt: String,
    #[serde(default)]
    draft: bool,
}

#[derive(Serialize, Clone)]
struct Article {
    slug: String,
    key: String,
    title: String,
    description: String,
    date: String,
    date_human: String,
    tags: Vec<String>,
    serie: String,
    author: String,
    image: Option<String>,
    image_alt: String,
    reading_minutes: usize,
    /// Path inside the site, e.g. "en/magazine/welcome/".
    path: String,
    href: String,
    abs: String,
    html: String,
}

#[derive(Serialize)]
struct Link {
    id: String,
    label: String,
    href: String,
    current: bool,
}

#[derive(Serialize)]
struct Alternate {
    code: String,
    label: String,
    name: String,
    hreflang: String,
    href: String,
    abs: String,
    current: bool,
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

struct Site<'a> {
    config: Config,
    locales: Vec<Locale>,
    /// Articles per locale, newest first.
    articles: Vec<Vec<Article>>,
    instagram: Vec<instagram::Post>,
    env: Environment<'a>,
    base: String,
    out: PathBuf,
    /// Every page written, for the sitemap.
    pages: Vec<String>,
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|s| s.replace("\r\n", "\n"))
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    toml::from_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn build(root: &Path, opts: &Options) -> Result<usize, String> {
    let config: Config = read_toml(&root.join("site.toml"))?;

    let mut locales = Vec::new();
    for code in LOCALES {
        let path = root.join("i18n").join(format!("{code}.toml"));
        let strings: toml::Value = read_toml(&path)?;
        let meta: LocaleMeta = strings
            .clone()
            .try_into()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for id in PAGES {
            if !meta.routes.contains_key(id) {
                return Err(format!("{}: [routes] is missing \"{id}\"", path.display()));
            }
        }
        let prefix = if code == "es" { String::new() } else { format!("{code}/") };
        locales.push(Locale { code, prefix, meta, strings });
    }

    for p in &config.projects {
        if !p.serie.is_empty() && !SERIES.contains(&p.serie.as_str()) {
            return Err(format!("site.toml: project {}: unknown serie \"{}\"", p.id, p.serie));
        }
        if p.url.is_none() && p.page.as_deref().is_none_or(|id| !PAGES.contains(&id)) {
            return Err(format!(
                "site.toml: project {} needs url = \"https://…\" or page = one of {}",
                p.id,
                PAGES.join(", ")
            ));
        }
    }

    let mut articles = Vec::new();
    for loc in &locales {
        articles.push(load_articles(root, loc, &config, &opts.base, opts.drafts)?);
    }

    let mut env = Environment::new();
    env.set_loader(path_loader(root.join("templates")));

    if opts.out.exists() {
        fs::remove_dir_all(&opts.out).map_err(|e| format!("cleaning {}: {e}", opts.out.display()))?;
    }
    copy_dir(&root.join("static"), &opts.out.join("static"))?;
    let instagram = instagram::load(root, &opts.out.join("static"), INSTAGRAM_POSTS)?;

    let mut site = Site {
        config,
        locales,
        articles,
        instagram,
        env,
        base: opts.base.clone(),
        out: opts.out.clone(),
        pages: Vec::new(),
    };

    for i in 0..site.locales.len() {
        site.render_locale(i)?;
    }
    site.render_404()?;
    site.write_sitemap()?;
    Ok(site.pages.len())
}

fn load_articles(
    root: &Path,
    loc: &Locale,
    config: &Config,
    base: &str,
    drafts: bool,
) -> Result<Vec<Article>, String> {
    let dir = root.join("content").join("articles").join(loc.code);
    let mut list = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(list);
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();

    let magazine = &loc.meta.routes["magazine"];
    for file in files {
        let slug = file.file_stem().unwrap().to_string_lossy().to_string();
        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "{}: use only lowercase letters, numbers and hyphens in the file name (it becomes the address)",
                file.display()
            ));
        }
        let text = read(&file)?;
        let (front, body) = split_front(&text)
            .ok_or_else(|| format!("{}: the article must start with a +++ block", file.display()))?;
        let front: Front = toml::from_str(front).map_err(|e| format!("{}: {e}", file.display()))?;
        if front.draft && !drafts {
            continue;
        }
        let date_human = human_date(&front.date, &loc.meta)
            .ok_or_else(|| format!("{}: date must look like 2026-11-08", file.display()))?;
        if !front.serie.is_empty() && !SERIES.contains(&front.serie.as_str()) {
            return Err(format!(
                "{}: serie must be one of {} (or left out for the palette of the day)",
                file.display(),
                SERIES.join(", ")
            ));
        }
        let path = format!("{}{magazine}/{slug}/", loc.prefix);
        list.push(Article {
            key: front.key.unwrap_or_else(|| slug.clone()),
            reading_minutes: reading_minutes(body, loc),
            html: markdown(body),
            href: format!("{base}{path}"),
            abs: format!("{}/{path}", config.url),
            path,
            slug,
            title: front.title,
            description: front.description,
            date: front.date,
            date_human,
            tags: front.tags,
            serie: front.serie,
            author: front.author.unwrap_or_else(|| {
                loc.strings["article"]["default_author"]
                    .as_str()
                    .unwrap_or("Mati Mati")
                    .to_string()
            }),
            image: front.image.map(|i| format!("{base}static/{i}")),
            image_alt: front.image_alt,
        });
    }
    // Newest first; same day → by title.
    list.sort_by(|a, b| b.date.cmp(&a.date).then(a.title.cmp(&b.title)));
    Ok(list)
}

fn split_front(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("+++\n")?;
    let end = rest.find("\n+++")?;
    let body = rest[end + 4..].trim_start_matches(['\n']);
    Some((&rest[..end], body))
}

fn markdown(body: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(body, opts));
    out
}

fn reading_minutes(body: &str, loc: &Locale) -> usize {
    let count = if loc.code == "zh" {
        body.chars().filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation()).count()
    } else {
        body.split_whitespace().count()
    };
    count.div_ceil(loc.meta.reading_rate.max(1)).max(1)
}

fn parse_date(date: &str) -> Option<(i64, u32, u32)> {
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn human_date(date: &str, meta: &LocaleMeta) -> Option<String> {
    let (y, m, d) = parse_date(date)?;
    let month = meta.months.get(m as usize - 1)?;
    Some(
        meta.date_format
            .replace("{month}", month)
            .replace("{d}", &d.to_string())
            .replace("{m}", &m.to_string())
            .replace("{y}", &y.to_string()),
    )
}

/// RFC 822 date for RSS, e.g. "Sun, 08 Nov 2026 12:00:00 +0000".
fn rfc822(date: &str) -> String {
    let Some((y, m, d)) = parse_date(date) else {
        return String::new();
    };
    // Days since 1970-01-01 (H. Hinnant's days_from_civil).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][days.rem_euclid(7) as usize];
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        [m as usize - 1];
    format!("{weekday}, {d:02} {month} {y} 12:00:00 +0000")
}

/// Encodes text for a URL query value (RFC 3986 unreserved kept as is).
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Site<'_> {
    fn abs(&self, path: &str) -> String {
        format!("{}/{path}", self.config.url)
    }

    /// Path of a page inside the site for a locale: "" for home,
    /// "en/magazine/" and so on.
    fn page_path(&self, li: usize, id: &str) -> String {
        let loc = &self.locales[li];
        if id == "home" {
            loc.prefix.clone()
        } else {
            format!("{}{}/", loc.prefix, loc.meta.routes[id])
        }
    }

    fn alternates(&self, li: usize, path_in: impl Fn(usize) -> String) -> Vec<Alternate> {
        self.locales
            .iter()
            .enumerate()
            .map(|(i, loc)| {
                let path = path_in(i);
                Alternate {
                    code: loc.code.into(),
                    label: loc.meta.label.clone(),
                    name: loc.meta.switch_name.clone(),
                    hreflang: loc.meta.hreflang.clone(),
                    href: format!("{}{path}", self.base),
                    abs: self.abs(&path),
                    current: i == li,
                }
            })
            .collect()
    }

    fn nav(&self, li: usize, current: &str) -> Vec<Link> {
        let t = &self.locales[li].strings["nav"];
        ["magazine", "tools", "about", "contact"]
            .iter()
            .map(|id| Link {
                id: id.to_string(),
                label: t[*id].as_str().unwrap_or(id).to_string(),
                href: format!("{}{}", self.base, self.page_path(li, id)),
                current: *id == current,
            })
            .collect()
    }

    /// Projects in this locale (texts fall back to Spanish).
    fn projects(&self, li: usize) -> Vec<Value> {
        let loc = &self.locales[li];
        let open = loc.strings["tools"]["open"].as_str().unwrap_or("Abrir");
        self.config
            .projects
            .iter()
            .filter_map(|p| {
                let text = p.text.get(loc.code).or_else(|| p.text.get("es"))?;
                let (url, external) = match (&text.url, &p.url, &p.page) {
                    (Some(u), _, _) | (None, Some(u), _) => (u.clone(), true),
                    (None, None, Some(id)) => {
                        (format!("{}{}", self.base, self.page_path(li, id)), false)
                    }
                    _ => return None,
                };
                Some(Value::from_serialize(context! {
                    id => p.id,
                    serie => p.serie,
                    art => p.art,
                    tool => p.tool,
                    url => url,
                    external => external,
                    name => text.name,
                    description => text.description,
                    tag => text.tag,
                    button => if text.button.is_empty() { open } else { &text.button },
                }))
            })
            .collect()
    }

    /// Topics bar: this locale's article tags, most used first, each linking
    /// to the magazine filtered by it.
    fn topics(&self, li: usize) -> Vec<Value> {
        let mut count: BTreeMap<&str, usize> = BTreeMap::new();
        for a in &self.articles[li] {
            for tag in &a.tags {
                *count.entry(tag).or_default() += 1;
            }
        }
        let mut tags: Vec<(&str, usize)> = count.into_iter().collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let magazine = format!("{}{}", self.base, self.page_path(li, "magazine"));
        tags.into_iter()
            .map(|(tag, _)| {
                Value::from_serialize(context! {
                    name => tag,
                    href => format!("{magazine}?tema={}", percent_encode(tag)),
                })
            })
            .collect()
    }

    /// Instagram posts with dates in this locale's format.
    fn posts(&self, li: usize) -> Vec<Value> {
        let meta = &self.locales[li].meta;
        self.instagram
            .iter()
            .map(|p| {
                let image = p.image.as_ref().map(|i| {
                    if i.starts_with("http") || i.starts_with('/') {
                        i.clone()
                    } else {
                        format!("{}static/{i}", self.base)
                    }
                });
                Value::from_serialize(context! {
                    url => p.url,
                    caption => p.caption,
                    kind => p.kind,
                    image => image,
                    date => p.date,
                    date_human => p.date.as_deref().and_then(|d| human_date(d, meta)),
                })
            })
            .collect()
    }

    fn write(&mut self, path: &str, html: String) -> Result<(), String> {
        let file = self.out.join(path).join("index.html");
        write_file(&file, html)?;
        self.pages.push(path.to_string());
        Ok(())
    }

    fn render(&self, template: &str, ctx: Value) -> Result<String, String> {
        let tmpl = self
            .env
            .get_template(template)
            .map_err(|e| format!("template {template}: {e:#}"))?;
        tmpl.render(ctx).map_err(|e| format!("template {template}: {e:#}"))
    }

    /// Variables every template gets.
    fn common(&self, li: usize, id: &str, title: &str, description: &str, path: &str) -> Value {
        let loc = &self.locales[li];
        context! {
            site => Value::from_serialize(&self.config),
            t => Value::from_serialize(&loc.strings),
            loc => context! {
                code => loc.code,
                html_lang => loc.meta.html_lang,
                og_locale => loc.meta.og_locale,
                prefix => loc.prefix,
            },
            base => self.base,
            home => format!("{}{}", self.base, loc.prefix),
            feed => format!("{}{}feed.xml", self.base, loc.prefix),
            nav => self.nav(li, id),
            links => context! {
                magazine => format!("{}{}", self.base, self.page_path(li, "magazine")),
                tools => format!("{}{}", self.base, self.page_path(li, "tools")),
                about => format!("{}{}", self.base, self.page_path(li, "about")),
                support => format!("{}{}", self.base, self.page_path(li, "support")),
                contact => format!("{}{}", self.base, self.page_path(li, "contact")),
                instagram => format!("https://www.instagram.com/{}/", self.config.instagram),
            },
            page => context! {
                id => id,
                title => title,
                description => description,
                path => path,
                abs => self.abs(path),
            },
            topics => self.topics(li),
            series => SERIES,
            logo_abs => self.abs("static/img/mati-logo-textil.png"),
            year => self.articles[li].first().map(|a| &a.date[..4]).unwrap_or("2026"),
        }
    }

    fn render_locale(&mut self, li: usize) -> Result<(), String> {
        let strings = &self.locales[li].strings;
        let text = |section: &str, key: &str| -> String {
            strings[section][key].as_str().unwrap_or_default().to_string()
        };

        // Every tag in this locale's articles, for the magazine filter.
        let tags: BTreeSet<String> = self.articles[li]
            .iter()
            .flat_map(|a| a.tags.iter().cloned())
            .collect();

        let mut queue: Vec<(String, String, Value)> = Vec::new();

        // Home
        let path = self.page_path(li, "home");
        let ctx = context! {
            articles => Value::from_serialize(self.articles[li].iter().take(5).collect::<Vec<_>>()),
            projects => self.projects(li),
            posts => self.posts(li),
            alternates => Value::from_serialize(self.alternates(li, |i| self.page_path(i, "home"))),
            ..self.common(li, "home", "", &text("home", "description"), &path)
        };
        queue.push(("home.html".into(), path, ctx));

        // Fixed pages
        for id in PAGES {
            let path = self.page_path(li, id);
            let section = match id {
                "magazine" => "magazine",
                other => other,
            };
            let ctx = context! {
                articles => Value::from_serialize(&self.articles[li]),
                tags => Value::from_serialize(&tags),
                projects => self.projects(li),
                alternates => Value::from_serialize(self.alternates(li, |i| self.page_path(i, id))),
                ..self.common(li, id, &text(section, "title"), &text(section, "description"), &path)
            };
            queue.push((format!("{id}.html"), path, ctx));
        }

        // Articles
        for (ai, article) in self.articles[li].iter().enumerate() {
            // The same article in each locale, or that locale's magazine.
            let alternates = self.alternates(li, |i| {
                self.articles[i]
                    .iter()
                    .find(|a| a.key == article.key)
                    .map(|a| a.path.clone())
                    .unwrap_or_else(|| self.page_path(i, "magazine"))
            });
            let translated: Vec<&str> = self
                .locales
                .iter()
                .enumerate()
                .filter(|(i, _)| self.articles[*i].iter().any(|a| a.key == article.key))
                .map(|(_, l)| l.code)
                .collect();
            // Two more articles to keep reading: the next ones, wrapping.
            let n = self.articles[li].len();
            let more: Vec<&Article> = (1..n)
                .map(|k| &self.articles[li][(ai + k) % n])
                .take(2)
                .collect();
            let ctx = context! {
                article => Value::from_serialize(article),
                more => Value::from_serialize(more),
                translated => translated,
                serie => article.serie,
                alternates => Value::from_serialize(alternates),
                ..self.common(li, "article", &article.title, &article.description, &article.path)
            };
            queue.push(("article.html".into(), article.path.clone(), ctx));
        }

        for (template, path, ctx) in queue {
            let html = self.render(&template, ctx)?;
            self.write(&path, html)?;
        }

        self.write_feed(li)
    }

    fn render_404(&mut self) -> Result<(), String> {
        let ctx = context! {
            alternates => Value::from_serialize(self.alternates(0, |i| self.page_path(i, "home"))),
            ..self.common(0, "404", "404", "", "404.html")
        };
        let html = self.render("404.html", ctx)?;
        write_file(&self.out.join("404.html"), html)
    }

    /// RSS 2.0 of a locale's articles, with the full text, so a newsletter
    /// service can send each new article by email (RSS-to-email).
    fn write_feed(&mut self, li: usize) -> Result<(), String> {
        let loc = &self.locales[li];
        let title = format!(
            "{} · {}",
            self.config.name,
            loc.strings["magazine"]["title"].as_str().unwrap_or("Revista")
        );
        let description = loc.strings["magazine"]["description"].as_str().unwrap_or_default();
        let home = self.abs(&loc.prefix);
        let feed = self.abs(&format!("{}feed.xml", loc.prefix));
        let mut xml = String::new();
        let _ = write!(
            xml,
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
             <rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n\
             <channel>\n<title>{}</title>\n<link>{}</link>\n<description>{}</description>\n<language>{}</language>\n\
             <atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\"/>\n",
            xml_escape(&title),
            xml_escape(&home),
            xml_escape(description),
            loc.meta.hreflang,
            xml_escape(&feed),
        );
        if let Some(first) = self.articles[li].first() {
            let _ = writeln!(xml, "<lastBuildDate>{}</lastBuildDate>", rfc822(&first.date));
        }
        for a in &self.articles[li] {
            let _ = write!(
                xml,
                "<item>\n<title>{}</title>\n<link>{}</link>\n<guid isPermaLink=\"true\">{}</guid>\n\
                 <pubDate>{}</pubDate>\n<description>{}</description>\n",
                xml_escape(&a.title),
                xml_escape(&a.abs),
                xml_escape(&a.abs),
                rfc822(&a.date),
                xml_escape(&a.description),
            );
            for tag in &a.tags {
                let _ = writeln!(xml, "<category>{}</category>", xml_escape(tag));
            }
            let _ = write!(
                xml,
                "<content:encoded><![CDATA[{}]]></content:encoded>\n</item>\n",
                a.html.replace("]]>", "]]]]><![CDATA[>")
            );
        }
        xml.push_str("</channel>\n</rss>\n");
        write_file(&self.out.join(format!("{}feed.xml", loc.prefix)), xml)
    }

    fn write_sitemap(&self) -> Result<(), String> {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for path in &self.pages {
            let _ = writeln!(xml, "<url><loc>{}</loc></url>", xml_escape(&self.abs(path)));
        }
        xml.push_str("</urlset>\n");
        write_file(&self.out.join("sitemap.xml"), xml)?;
        write_file(
            &self.out.join("robots.txt"),
            format!("User-agent: *\nAllow: /\n\nSitemap: {}\n", self.abs("sitemap.xml")),
        )
    }
}

fn write_file(file: &Path, contents: String) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(file, contents).map_err(|e| format!("{}: {e}", file.display()))
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Local server (--serve)
// ---------------------------------------------------------------------------

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn serve(dir: &Path, base: &str, port: u16) {
    let server = tiny_http::Server::http(("127.0.0.1", port)).expect("could not bind port");
    println!("Mati Mati → http://127.0.0.1:{port}{base}");
    for request in server.incoming_requests() {
        let url = request
            .url()
            .split(['?', '#'])
            .next()
            .unwrap_or("/")
            .to_string();
        let url = percent_decode(&url);
        if url == "/" && base != "/" {
            let header = tiny_http::Header::from_bytes("Location", base.as_bytes()).unwrap();
            let _ = request.respond(tiny_http::Response::empty(302).with_header(header));
            continue;
        }
        let inside = url
            .strip_prefix(base.trim_end_matches('/'))
            .filter(|r| r.is_empty() || r.starts_with('/'));
        let safe = inside.is_some() && !url.split('/').any(|seg| seg == "..");
        let mut path = dir.join(inside.unwrap_or("").trim_start_matches('/'));
        if safe && path.is_dir() && !url.ends_with('/') {
            let to = format!("{url}/");
            let header = tiny_http::Header::from_bytes("Location", to.as_bytes()).unwrap();
            let _ = request.respond(tiny_http::Response::empty(301).with_header(header));
            continue;
        }
        if path.is_dir() {
            path = path.join("index.html");
        }
        let (status, file) = if safe && path.is_file() {
            (200, path)
        } else {
            (404, dir.join("404.html"))
        };
        let body = fs::read(&file).unwrap_or_default();
        let header = tiny_http::Header::from_bytes("Content-Type", content_type(&file)).unwrap();
        let _ = request.respond(
            tiny_http::Response::from_data(body)
                .with_status_code(status)
                .with_header(header),
        );
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_dates() {
        assert_eq!(rfc822("2026-11-08"), "Sun, 08 Nov 2026 12:00:00 +0000");
        assert_eq!(rfc822("2000-01-01"), "Sat, 01 Jan 2000 12:00:00 +0000");
        assert_eq!(rfc822("2024-02-29"), "Thu, 29 Feb 2024 12:00:00 +0000");
    }

    #[test]
    fn front_matter() {
        let (front, body) = split_front("+++\ntitle = \"x\"\n+++\n\nHola").unwrap();
        assert_eq!(front, "title = \"x\"");
        assert_eq!(body, "Hola");
        assert!(split_front("Hola").is_none());
    }

    #[test]
    fn decode() {
        assert_eq!(percent_decode("/a%20b/"), "/a b/");
        assert_eq!(percent_decode("/100%"), "/100%");
    }
}
