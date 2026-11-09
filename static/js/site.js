// Mati Mati · small enhancements. Every page works without JavaScript;
// this only adds the magazine filter, the share button and lazy Instagram.

// ---------- Magazine: filter articles by topic ----------
const filters = document.querySelector("[data-filters]");
const list = document.querySelector("[data-articles]");
if (filters && list) {
  const cards = [...list.querySelectorAll(".article-card")];
  const count = filters.querySelector("[data-count]");
  const buttons = [...filters.querySelectorAll("[data-filter]")];

  const apply = (tag) => {
    let shown = 0;
    for (const card of cards) {
      const tags = (card.dataset.tags || "").split("|");
      const visible = !tag || tags.includes(tag);
      card.hidden = !visible;
      if (visible) shown++;
    }
    for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.filter === tag));
    count.textContent = shown === 1 ? count.dataset.one : count.dataset.many.replace("{n}", shown);
    const url = new URL(location.href);
    if (tag) url.searchParams.set("tema", tag);
    else url.searchParams.delete("tema");
    history.replaceState(null, "", url);
  };

  filters.addEventListener("click", (ev) => {
    const button = ev.target.closest("[data-filter]");
    if (button) apply(button.dataset.filter);
  });
  filters.hidden = false;
  const initial = new URLSearchParams(location.search).get("tema") || "";
  apply(buttons.some((b) => b.dataset.filter === initial) ? initial : "");
}

// ---------- Article: share (native sheet on phones, copy link elsewhere) ----------
for (const button of document.querySelectorAll("[data-share]")) {
  if (!navigator.share && !navigator.clipboard) continue;
  button.hidden = false;
  const label = button.textContent;
  button.addEventListener("click", async () => {
    const url = location.href.split("#")[0];
    try {
      if (navigator.share) {
        await navigator.share({ title: button.dataset.title, url });
      } else {
        await navigator.clipboard.writeText(url);
        button.textContent = button.dataset.copied;
        setTimeout(() => (button.textContent = label), 2000);
      }
    } catch {
      /* the person closed the share sheet */
    }
  });
}

// ---------- Instagram: load the embeds only when the section is near ----------
// Instagram's script sets its own cookies, so it doesn't load until needed.
const instagram = document.querySelector("[data-instagram]");
if (instagram) {
  const load = () => {
    if (window.instgrm) return window.instgrm.Embeds.process();
    const s = document.createElement("script");
    s.src = "https://www.instagram.com/embed.js";
    s.async = true;
    document.body.append(s);
  };
  if ("IntersectionObserver" in window) {
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          io.disconnect();
          load();
        }
      },
      { rootMargin: "400px" },
    );
    io.observe(instagram);
  } else {
    load();
  }
}
