# Mati Mati · sitio principal

The main Mati Mati website: magazine, tools, about, support (Ko-fi) and contact, in Spanish (main language), English and Chinese. Built with the Mati Mati brand (Manual de Identidad Visual v2.0).

Live site: https://nojsuon.github.io/matimati/ (English: `/en/`, Chinese: `/zh/`)

## How it works

A small Rust program turns the content into static HTML for GitHub Pages:

| Folder / file | What it holds |
|---|---|
| `content/articles/es/`, `en/`, `zh/` | One Markdown file per article |
| `content/instagram.toml` | Instagram posts shown on the home page |
| `site.toml` | Name, email, Instagram, Ko-fi, tools, newsletter and ads settings |
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
serie = "mar"            # mati, mar, milpa, jacaranda, fiesta, anil, barro, noche
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
