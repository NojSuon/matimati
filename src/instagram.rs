//! Instagram posts for the home page.
//!
//! Three sources, in order:
//! 1. `INSTAGRAM_TOKEN` set (GitHub secret): the latest posts from the
//!    Instagram API. Two kinds of token work:
//!    - "IG…": Instagram Login token → graph.instagram.com/me/media
//!      (refreshed on every build, it lasts 60 days).
//!    - "EAA…": system-user token from "API setup with Unified login"
//!      (Facebook Login for Business; never expires) →
//!      graph.facebook.com/<account id>/media. The account id comes from
//!      `INSTAGRAM_USER_ID`, or is looked up by the handle in site.toml.
//!    Images are downloaded into the site, because Instagram's links expire.
//! 2. `INSTAGRAM_FIXTURE` set: a saved API response (JSON), for local
//!    previews without a token. Image links are used as they are.
//! 3. Otherwise: the posts listed by hand in content/instagram.toml.
//!
//! If the API fails, the build carries on with the hand-made list, so a
//! problem with Instagram never takes the site down.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

const API: &str = "https://graph.instagram.com";
const FB_API: &str = "https://graph.facebook.com";
const FIELDS: &str = "id,caption,media_type,media_url,thumbnail_url,permalink,timestamp";

/// A post, ready for the templates.
#[derive(Serialize, Clone, Default)]
pub struct Post {
    pub url: String,
    pub caption: String,
    /// Path inside static/ ("img/instagram/123.jpg"), or a full address.
    pub image: Option<String>,
    /// "YYYY-MM-DD".
    pub date: Option<String>,
    /// "image", "video" or "carousel".
    pub kind: String,
}

/// content/instagram.toml
#[derive(Deserialize, Default)]
pub struct Manual {
    #[serde(default)]
    posts: Vec<ManualPost>,
}

#[derive(Deserialize)]
struct ManualPost {
    url: String,
    #[serde(default)]
    caption: String,
    image: Option<String>,
    date: Option<String>,
}

/// One item of the API's /me/media answer.
#[derive(Deserialize)]
struct Media {
    id: String,
    #[serde(default)]
    caption: Option<String>,
    media_type: String,
    #[serde(default)]
    media_url: Option<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
    permalink: String,
    #[serde(default)]
    timestamp: String,
}

#[derive(Deserialize)]
struct MediaPage {
    data: Vec<Media>,
}

pub fn load(root: &Path, out_static: &Path, handle: &str, limit: usize) -> Result<Vec<Post>, String> {
    let token = std::env::var("INSTAGRAM_TOKEN").unwrap_or_default();
    let user_id = std::env::var("INSTAGRAM_USER_ID").unwrap_or_default();
    if !token.trim().is_empty() {
        let img_dir = out_static.join("img").join("instagram");
        match fetch(token.trim(), user_id.trim(), handle, limit, &img_dir) {
            Ok(posts) => {
                report("notice", &format!("{} posts from the API", posts.len()));
                return Ok(posts);
            }
            Err(e) => report(
                "warning",
                &format!("API failed, using content/instagram.toml ({e})"),
            ),
        }
    } else if std::env::var("GITHUB_ACTIONS").is_ok() {
        report("notice", "no INSTAGRAM_TOKEN secret, using content/instagram.toml");
    }
    if let Ok(path) = std::env::var("INSTAGRAM_FIXTURE") {
        let json = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let posts = parse(&json)?.into_iter().take(limit).map(|m| to_post(m, None)).collect();
        return Ok(posts);
    }
    let path = root.join("content").join("instagram.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manual: Manual = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(manual
        .posts
        .into_iter()
        .take(limit)
        .map(|p| Post {
            url: p.url,
            caption: p.caption,
            image: p.image,
            date: p.date,
            kind: "image".into(),
        })
        .collect())
}

/// Prints the Instagram result. On GitHub Actions it also becomes an
/// annotation, visible on the run's page without opening the log.
fn report(level: &str, message: &str) {
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        // Annotations are one line; % and newlines must be escaped.
        let msg = message.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A");
        println!("::{level} title=Instagram::{msg}");
    } else if level == "notice" {
        println!("Instagram: {message}");
    } else {
        eprintln!("{level}: Instagram {message}");
    }
}

fn parse(json: &str) -> Result<Vec<Media>, String> {
    serde_json::from_str::<MediaPage>(json)
        .map(|p| p.data)
        .map_err(|e| format!("unexpected Instagram answer: {e}"))
}

fn kind(media_type: &str) -> &'static str {
    match media_type {
        "VIDEO" => "video",
        "CAROUSEL_ALBUM" => "carousel",
        _ => "image",
    }
}

fn to_post(m: Media, image: Option<String>) -> Post {
    let image = image.or_else(|| {
        if m.media_type == "VIDEO" {
            m.thumbnail_url.clone()
        } else {
            m.media_url.clone()
        }
    });
    Post {
        caption: short_caption(m.caption.as_deref().unwrap_or("")),
        date: m.timestamp.get(..10).map(str::to_string),
        kind: kind(&m.media_type).into(),
        url: m.permalink,
        image,
    }
}

/// First line of the caption, without trailing hashtags, at most ~140 chars.
pub fn short_caption(caption: &str) -> String {
    let line = caption.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let words: Vec<&str> = line.split_whitespace().collect();
    let end = words.iter().rposition(|w| !w.starts_with('#')).map_or(0, |i| i + 1);
    let mut text = words[..end].join(" ");
    if text.chars().count() > 140 {
        text = text.chars().take(139).collect::<String>();
        if let Some(cut) = text.rfind(' ') {
            text.truncate(cut);
        }
        text = text.trim_end_matches([',', '.', ';', ':']).to_string() + "…";
    }
    text
}

/// Which API a token belongs to.
#[derive(Debug, PartialEq)]
enum TokenKind {
    /// Instagram Login ("IG…"): graph.instagram.com, /me is the account.
    Instagram,
    /// Facebook Login for Business / system user ("EAA…"): graph.facebook.com,
    /// the Instagram account is addressed by its id.
    Facebook,
}

fn token_kind(token: &str) -> TokenKind {
    if token.starts_with("IG") {
        TokenKind::Instagram
    } else {
        TokenKind::Facebook
    }
}

/// Error text without the token (it travels in the address).
fn clean(token: &str, e: ureq::Error) -> String {
    let text = match e {
        // Meta explains what went wrong in the body: show it.
        ureq::Error::Status(code, resp) => {
            format!("HTTP {code}: {}", resp.into_string().unwrap_or_default())
        }
        other => other.to_string(),
    };
    text.replace(token, "***")
}

/// The Instagram account id for a Facebook-type token: INSTAGRAM_USER_ID, or
/// the account with this handle among those the token can see.
fn account_id(token: &str, user_id: &str, handle: &str) -> Result<String, String> {
    if !user_id.is_empty() {
        if !user_id.chars().all(|c| c.is_ascii_digit()) {
            return Err("INSTAGRAM_USER_ID must be the account's number".into());
        }
        return Ok(user_id.to_string());
    }
    #[derive(Deserialize)]
    struct Pages {
        data: Vec<Page>,
    }
    #[derive(Deserialize)]
    struct Page {
        instagram_business_account: Option<Account>,
    }
    #[derive(Deserialize)]
    struct Account {
        id: String,
        #[serde(default)]
        username: String,
    }
    let pages: Pages = ureq::get(&format!("{FB_API}/me/accounts"))
        .query("fields", "instagram_business_account{id,username}")
        .query("limit", "100")
        .query("access_token", token)
        .call()
        .map_err(|e| clean(token, e))?
        .into_json()
        .map_err(|e| format!("unexpected answer: {e}"))?;
    pages
        .data
        .into_iter()
        .filter_map(|p| p.instagram_business_account)
        .find(|a| a.username.eq_ignore_ascii_case(handle))
        .map(|a| a.id)
        .ok_or_else(|| {
            format!("couldn't find @{handle} with this token; set the INSTAGRAM_USER_ID variable")
        })
}

fn fetch(
    token: &str,
    user_id: &str,
    handle: &str,
    limit: usize,
    img_dir: &Path,
) -> Result<Vec<Post>, String> {
    let url = match token_kind(token) {
        TokenKind::Instagram => {
            // Long-lived tokens last 60 days; refreshing extends them. Best
            // effort: it only works once the token is at least a day old.
            let _ = ureq::get(&format!("{API}/refresh_access_token"))
                .query("grant_type", "ig_refresh_token")
                .query("access_token", token)
                .call();
            format!("{API}/me/media")
        }
        TokenKind::Facebook => format!("{FB_API}/{}/media", account_id(token, user_id, handle)?),
    };

    let page: MediaPage = ureq::get(&url)
        .query("fields", FIELDS)
        .query("limit", &limit.to_string())
        .query("access_token", token)
        .call()
        .map_err(|e| clean(token, e))?
        .into_json()
        .map_err(|e| format!("unexpected Instagram answer: {e}"))?;

    fs::create_dir_all(img_dir).map_err(|e| format!("{}: {e}", img_dir.display()))?;
    let mut posts = Vec::new();
    for m in page.data {
        let src = if m.media_type == "VIDEO" { &m.thumbnail_url } else { &m.media_url };
        let mut image = None;
        if let Some(src) = src {
            // Ids are digits; anything else is not used as a file name.
            if m.id.chars().all(|c| c.is_ascii_digit()) {
                let file = img_dir.join(format!("{}.jpg", m.id));
                match download(src, &file) {
                    Ok(()) => image = Some(format!("img/instagram/{}.jpg", m.id)),
                    Err(e) => eprintln!("warning: Instagram image {}: {e}", m.id),
                }
            }
        }
        // Without a local copy the post still shows, as a text card.
        posts.push(to_post(Media { media_url: None, thumbnail_url: None, ..m }, image));
    }
    Ok(posts)
}

fn download(url: &str, file: &Path) -> Result<(), String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader().take(15 * 1024 * 1024);
    let mut out = fs::File::create(file).map_err(|e| e.to_string())?;
    io::copy(&mut reader, &mut out).map_err(|e| e.to_string())?;
    Ok(())
}

use std::io::Read as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_api_answer() {
        let json = r##"{"data":[
            {"id":"1","caption":"Hola #nahuatl","media_type":"IMAGE","media_url":"https://x/a.jpg","permalink":"https://www.instagram.com/p/A/","timestamp":"2026-11-07T18:00:00+0000"},
            {"id":"2","media_type":"VIDEO","thumbnail_url":"https://x/t.jpg","media_url":"https://x/v.mp4","permalink":"https://www.instagram.com/reel/B/","timestamp":"2026-11-06T18:00:00+0000"}
        ],"paging":{}}"##;
        let posts: Vec<Post> = parse(json).unwrap().into_iter().map(|m| to_post(m, None)).collect();
        assert_eq!(posts[0].caption, "Hola");
        assert_eq!(posts[0].date.as_deref(), Some("2026-11-07"));
        assert_eq!(posts[1].kind, "video");
        assert_eq!(posts[1].image.as_deref(), Some("https://x/t.jpg"));
    }

    #[test]
    fn token_kinds() {
        assert_eq!(token_kind("IGAAbc"), TokenKind::Instagram);
        assert_eq!(token_kind("EAAbc"), TokenKind::Facebook);
    }

    #[test]
    fn explicit_account_id() {
        assert_eq!(account_id("EAA", "17841400000000000", "x").unwrap(), "17841400000000000");
        assert!(account_id("EAA", "../me", "x").is_err());
    }

    #[test]
    fn captions() {
        assert_eq!(short_caption("\n\nPalabra del día\nmás texto"), "Palabra del día");
        assert_eq!(short_caption("Gracias #a #b"), "Gracias");
        let long = "palabra ".repeat(40);
        let s = short_caption(&long);
        assert!(s.ends_with('…') && s.chars().count() <= 141);
    }
}
