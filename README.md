# Mati Mati · sitio principal

The main Mati Mati website: magazine, tools, about, support (Ko-fi) and contact, in Spanish (main language), English and Chinese. Built with the Mati Mati brand (Manual de Identidad Visual v2.0).

Live site: https://nojsuon.github.io/matimati/ (English: `/en/`, Chinese: `/zh/`)

## How it works

A small Rust program turns the content into static HTML for GitHub Pages:

| Folder / file | What it holds |
|---|---|
| `content/articles/es/`, `en/`, `zh/` | One Markdown file per article |
| `content/instagram.toml` | Instagram posts by hand (used only without the API token) |
| `site.toml` | Name, email, Instagram, Ko-fi, projects (home carousel), newsletter and ads |
| `i18n/es.toml`, `en.toml`, `zh.toml` | Every interface text, per language |
| `templates/` | HTML templates (MiniJinja, Jinja-like syntax) |
| `static/` | CSS (brand tokens), JavaScript, images |
| `src/main.rs` | The generator |

It also publishes an RSS feed per language (`/feed.xml`, `/en/feed.xml`, `/zh/feed.xml`), plus `sitemap.xml` and `robots.txt`.

## Write an article

1. Create `content/articles/es/mi-articulo.md`. The file name becomes the address (`/revista/mi-articulo/`): lowercase letters, numbers and hyphens only.
2. Start it with this block, then write the text in Markdown:

```markdown
+++
title = "Título del artículo"
description = "Una o dos frases: aparece en la tarjeta, en buscadores y en el RSS."
date = "2026-11-20"
key = "mi-articulo"      # the same key in the en/ and zh/ translations links them
tags = ["Náhuatl", "Palabras"]
serie = "mar"            # optional: mati, mar, milpa, jacaranda, fiesta, anil, barro, noche (default: palette of the day)
# author = "Nombre"      # default: Equipo Mati Mati
# image = "img/articulos/mi-foto.jpg"   # optional, inside static/
# image_alt = "Descripción de la imagen"
# draft = true           # hidden until you remove this line
+++

Texto del artículo…
```

3. Optional: translate it into `content/articles/en/` and `content/articles/zh/` with the same `key`. The language switch then links the versions; without a translation it points to that language's magazine.

Inside the text you can use:

- A word in an indigenous language (brand style): `<span class="lang" lang="nah">Tlasojkamati</span>`
- A highlighted word of the day: `<p class="word"><span class="lang" lang="nah">Tlasojkamati</span> «gracias» en náhuatl</p>`

Words in indigenous languages should be checked with speakers before publishing (Manual §02).

## Instagram, automatically

Every day at 6:00 (Mexico City) the site rebuilds and pulls the latest 6 posts from @matidemati through the official Instagram API. Images are copied into the site, so they keep working. One-time setup:

1. The Instagram account must be **professional** (Business or Creator): in the app, Settings → Account type and tools.
2. At https://developers.facebook.com create an app with the use case **Manage messaging & content on Instagram**, connect a business portfolio, and under **API setup with Unified login** create a configuration (any redirect URL, e.g. this site's address). In **2. Generate access tokens**, add @matidemati (it needs the *Instagram Tester* role under App roles → Roles while the app is unpublished) and generate a token.
3. In this repository: **Settings → Secrets and variables → Actions → New repository secret**, name `INSTAGRAM_TOKEN`, value: the token. Both kinds work: `IG…` tokens (Instagram Login, refreshed on every build) and `EAA…` system-user tokens (never expire). With an `EAA…` token, if the build log says it couldn't find the account, also add a repository **variable** (Variables tab) `INSTAGRAM_USER_ID` with the account's number from the Meta dashboard.
4. Run the workflow once (Actions → Deploy to GitHub Pages → Run workflow).

Each build also refreshes the token's 60-day validity, so as long as the site rebuilds daily it keeps working. If the API fails, the build doesn't: the home page falls back to `content/instagram.toml`.

To preview the grid without a token, point `INSTAGRAM_FIXTURE` at a saved API answer (JSON with a `data` list).

## Look and feel

- **Palette of the day:** each day the site takes one of the manual's eight colour series (mati, mar, milpa, jacaranda, fiesta, anil, barro, noche), the same for everyone on that date. Preview one with `?paleta=milpa`. One colour family per page (Manual §04): an article with its own `serie` uses it on its own page only; in lists it takes the palette of the day.
- **Light by default.** Dark mode only when chosen with the moon/sun button (remembered). It uses only brand colours: Tinta background, Grafito surfaces, Hueso text and each series' dark solid.
- **HUD style** (as in Chinamitl and the conjugator): a floating header of frosted pill panels, pill buttons with soft shadows, hairline borders and rounded cards (28 / 44 px).
- **Pattern:** P11 Estrella de ocho puntas (`static/img/patron-estrella.svg`), recoloured by the palette. Only in small blocks, never under text (§06).
- **Textile logo:** only on the about page hero, on a plain background (§03). Header and footer use the simple monogram.
- **Home carousel:** the `[[projects]]` in `site.toml`, in order. Each shows its `image`: a screenshot in `static/img/proyectos/` (JPEG, 1440 px wide, 16:10), or `"instagram"` for the latest Instagram post. To refresh a screenshot, replace the file.

## Run it locally

You need Rust. Build and serve the site at http://127.0.0.1:3000/matimati/:

```bash
cargo run -- --base matimati --serve
```

Add `--drafts` to also see articles marked `draft = true`. Run the tests with `cargo test`.

## Publishing

Every push to `main` builds the site and publishes it to GitHub Pages (`.github/workflows/pages.yml`).

One-time setup: in the repository, **Settings → Pages → Build and deployment → Source: GitHub Actions**.

## Next steps (already prepared)

- **Newsletter:** the RSS feeds are live. Connect them to MailerLite or Buttondown ("RSS to email") and set `[newsletter]` in `site.toml` to show the sign-up form.
- **Advertising:** needs the site's own domain first (AdSense does not accept `*.github.io`). Then fill `[ads]` in `site.toml`; until then no ad code loads.
- **Own domain:** change `url` in `site.toml` and set the domain in **Settings → Pages → Custom domain**.
