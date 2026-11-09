// Mati Mati · enhancements. Every page works without JavaScript; this adds
// the theme switch, the carousel controls, the magazine filter and sharing.
// The palette of the day and the saved theme are applied earlier, by the
// inline script in <head> (templates/base.html), so nothing flashes.

const root = document.documentElement;

// ---------- Light / dark ----------
const STORE = "mati-tema";
const toggle = document.querySelector("[data-theme-toggle]");
const metaTheme = document.querySelector('meta[name="theme-color"]');
function showTheme() {
  const dark = root.dataset.theme === "dark";
  if (toggle) {
    const label = dark ? toggle.dataset.labelLight : toggle.dataset.labelDark;
    toggle.setAttribute("aria-label", label);
    toggle.title = label;
  }
  metaTheme?.setAttribute("content", dark ? "#1E1512" : "#FFF9F5");
}
toggle?.addEventListener("click", () => {
  root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
  try {
    localStorage.setItem(STORE, root.dataset.theme);
  } catch {
    /* private mode: the choice lasts for this page only */
  }
  showTheme();
});
// Follow the system while the person hasn't chosen.
matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", (e) => {
  let saved = null;
  try {
    saved = localStorage.getItem(STORE);
  } catch {}
  if (!saved) {
    root.dataset.theme = e.matches ? "dark" : "light";
    showTheme();
  }
});
showTheme();

// ---------- Palette of the day (footer label) ----------
const palette = document.querySelector("[data-palette]");
if (palette) {
  try {
    const names = JSON.parse(palette.dataset.names);
    palette.textContent = `${palette.dataset.label}: ${names[root.dataset.serie] ?? root.dataset.serie}`;
  } catch {}
}

// ---------- Projects carousel ----------
const carousel = document.querySelector("[data-carousel]");
if (carousel) {
  const viewport = carousel.querySelector("[data-viewport]");
  const slides = [...viewport.children];
  const controls = carousel.querySelector("[data-controls]");
  const dots = [...carousel.querySelectorAll("[data-dot]")];
  const pauseBtn = carousel.querySelector("[data-pause]");
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  let current = 0;
  let playing = !reduced && slides.length > 1;
  let hovering = false;
  let timer = null;

  const go = (i) => {
    current = (i + slides.length) % slides.length;
    viewport.scrollTo({ left: current * viewport.clientWidth, behavior: reduced ? "auto" : "smooth" });
    mark();
  };
  const mark = () => {
    dots.forEach((d, i) => d.setAttribute("aria-current", String(i === current)));
    slides.forEach((s, i) => s.toggleAttribute("inert", i !== current));
  };
  // Keep the dots in sync when the person swipes or scrolls.
  let scrollTimer = null;
  viewport.addEventListener("scroll", () => {
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => {
      current = Math.round(viewport.scrollLeft / viewport.clientWidth);
      mark();
    }, 80);
  });
  const schedule = () => {
    clearInterval(timer);
    if (playing && !hovering) timer = setInterval(() => go(current + 1), 7000);
  };
  const showPause = () => {
    if (!pauseBtn) return;
    const label = playing ? carousel.dataset.labelPause : carousel.dataset.labelPlay;
    pauseBtn.textContent = playing ? "❚❚" : "▶";
    pauseBtn.setAttribute("aria-label", label);
    pauseBtn.title = label;
  };

  if (controls) {
    controls.hidden = false;
    carousel.querySelector("[data-prev]").addEventListener("click", () => go(current - 1));
    carousel.querySelector("[data-next]").addEventListener("click", () => go(current + 1));
    dots.forEach((d, i) => d.addEventListener("click", () => go(i)));
    pauseBtn?.addEventListener("click", () => {
      playing = !playing;
      showPause();
      schedule();
    });
    // Pause while the pointer or keyboard focus is inside (WCAG 2.2.2).
    carousel.addEventListener("pointerenter", () => ((hovering = true), schedule()));
    carousel.addEventListener("pointerleave", () => ((hovering = false), schedule()));
    carousel.addEventListener("focusin", () => ((hovering = true), schedule()));
    carousel.addEventListener("focusout", () => ((hovering = false), schedule()));
    viewport.addEventListener("keydown", (e) => {
      if (e.key === "ArrowRight") (e.preventDefault(), go(current + 1));
      if (e.key === "ArrowLeft") (e.preventDefault(), go(current - 1));
    });
    showPause();
    mark();
    schedule();
  }
}

// ---------- Magazine: filter articles by topic (?tema=…) ----------
const filters = document.querySelector("[data-filters]");
const list = document.querySelector("[data-articles]");
if (filters && list) {
  const items = [...list.children];
  const count = filters.querySelector("[data-count]");
  const buttons = [...filters.querySelectorAll("[data-filter]")];

  const apply = (tag) => {
    let shown = 0;
    for (const item of items) {
      const visible = !tag || (item.dataset.tags || "").split("|").includes(tag);
      item.hidden = !visible;
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
