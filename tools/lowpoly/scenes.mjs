// Low-poly illustrations of each Mati Mati project, in brand colours.
// Run:  node tools/lowpoly/scenes.mjs   → static/img/proyectos/<id>.svg
import { writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Scene, v, rng, mix, ring } from "./engine.mjs";

// Manual de Identidad Visual v2.0, §04.
const C = {
  tinta: "#2B1D1A", papel: "#FFF9F5", hueso: "#F0EAE6", lino: "#B8A890", algodon: "#E4DFD8", piedra: "#7A6B62",
  terracota: "#C1502E", arcilla: "#D07050", cacao: "#7A2E1D", mostaza: "#D9A93F", ocre: "#9C5400", miel: "#C98A2E",
  caramelo: "#A5612C", cafe: "#5A3F33", turquesa: "#44989D", jade: "#3A7A6D", petroleo: "#2F6B73", espuma: "#CDDFDD",
  nopal: "#55803F", pino: "#2F5D4E", lima: "#A3AE45", salvia: "#6E9F8A", anil: "#2E4A7D", talavera: "#4A72A8",
  cielo: "#7FA6CC", jacaranda: "#7A63A8", berenjena: "#4A2F5B", bugambilia: "#C93A80", sandia: "#D9556B",
  cempasuchil: "#DE7D2A", arena: "#E4CCB3", trigo: "#E8D3C3", rosa: "#FFE7E0", lila: "#E8E4F1", maiz: "#F4E2C0",
};

const scenes = {};

// ---------- Chinamitl: a chinampa on the lake, volcanoes behind ----------
scenes.chinamitl = () => {
  const r = rng(7), s = new Scene();
  s.surface(-70, -50, 140, 110, 28, 22,
    (x, y) => Math.sin(x * 0.35 + y * 0.2) * 0.35 + (r() - 0.5) * 0.5,
    () => mix(C.petroleo, C.turquesa, r() * 0.7), { layer: 0 });
  // Volcanoes (Popocatépetl, Iztaccíhuatl) with snow.
  for (const [x, y, rad, h] of [[-38, 52, 26, 30], [6, 58, 34, 24], [42, 50, 22, 20]]) {
    s.cone(x, y, 0, rad, h, 7, mix(C.lino, C.piedra, r() * 0.4), { layer: 0.5, phase: r() });
    s.cone(x, y, h * 0.72, rad * 0.3, h * 0.28, 7, C.hueso, { layer: 0.6, phase: r() });
  }
  // Neighbour chinampas.
  for (const [x, y, w, d] of [[-62, 20, 34, 13], [8, 22, 38, 12], [36, -38, 26, 11]]) {
    s.box(x, y, -0.6, w, d, 2.4, C.cafe, { top: C.nopal });
    for (let i = 0; i < 3; i++) s.box(x + 2, y + 2 + i * 3.3, 1.8, w - 4, 1.6, 0.6, i % 2 ? C.mostaza : C.lima, { layer: 1.2 });
  }
  // The main chinampa: earth, crop rows, stakes.
  s.box(-26, -14, -0.8, 52, 28, 3.4, C.cafe, { top: C.nopal });
  const rows = [C.lima, C.mostaza, C.nopal, C.lima, C.cempasuchil, C.lima];
  rows.forEach((c, i) => s.box(-4, -11 + i * 4, 2.6, 26, 2.4, 0.9, c, { layer: 2 }));
  for (let x = -25; x <= 25; x += 5) {
    s.box(x, -14.8, -0.6, 1, 1, 4, C.cacao, { layer: 1.5 });
  }
  // Ahuejotes along the long edges.
  for (const y of [-12, 12]) {
    for (let x = -22; x <= 22; x += 6.5) {
      s.cylinder(x, y, 2.6, 0.5, 3, 5, C.cafe, { layer: 2 });
      s.blob(x, y, 11, 2.3, 2.3, 7, r() > 0.5 ? C.pino : C.nopal, { layer: 2.1, rand: r, jitter: 0.12 });
    }
  }
  // House with a thatched roof.
  s.box(-21, -6, 2.6, 9, 8, 5, C.arena, { layer: 2 });
  s.cone(-16.5, -2, 7.6, 7.2, 5.5, 4, C.miel, { layer: 2.2, phase: Math.PI / 4 });
  // Canoe.
  const canoe = ring(18, -30, 7, 10, 0, 1, 0.24);
  s.prism(canoe, -0.2, 1.4, C.caramelo, { top: C.cafe, layer: 1 });
  s.cylinder(15, -30, 1.2, 0.3, 7, 4, C.cafe, { layer: 1.1 });
  return s.svg({ yaw: -28, pitch: 30, title: "Chinamitl" });
};

// ---------- Buul: the race board, pieces and corn kernels as dice ----------
scenes.buul = () => {
  const r = rng(11), s = new Scene();
  s.box(-36, -14, 0, 72, 28, 2.2, C.cafe, { top: C.caramelo });
  const cols = 11, rowsN = 5, tw = 5.6, td = 4.4;
  for (let i = 0; i < cols; i++) {
    for (let j = 0; j < rowsN; j++) {
      const x = -34 + i * (tw + 0.6), y = -12 + j * (td + 0.8);
      s.box(x, y, 2.2, tw, td, 0.5, i % 2 ? C.lino : C.hueso, { layer: 2 });
    }
  }
  const piece = (i, j, color) => s.cylinder(-34 + i * 6.2 + 2.8, -12 + j * 5.2 + 2.2, 2.7, 1.8, 1.6, 9, color, { layer: 3, top: mix(color, C.papel, 0.15) });
  for (const j of [0, 2, 3, 4]) piece(0, j, C.petroleo);
  piece(3, 1, C.petroleo);
  for (const j of [0, 1, 3, 4]) piece(10, j, C.cempasuchil);
  piece(6, 2, C.cempasuchil);
  // Corn kernels: two burnt on the side facing up.
  [[-18, -24, 30, 0], [-6, -27, -20, 1], [6, -23, 60, 0], [18, -26, 10, 1]].forEach(([x, y, rot, burnt]) => {
    s.blob(x, y, 2, 3.2, 4.6, 2, burnt ? C.cafe : C.maiz, { layer: 1.5, rand: r, rot, jitter: 0.1 });
  });
  return s.svg({ yaw: -18, pitch: 42, title: "Buul" });
};

// ---------- Conjugador: word blocks (prefix · root · ending) and a speech bubble ----------
scenes.conjugador = () => {
  const r = rng(5), s = new Scene();
  // Speech bubble standing behind.
  const bubble = [];
  for (let i = 0; i <= 12; i++) {
    const a = Math.PI + (i / 12) * Math.PI; // top half of a stadium
    bubble.push([Math.cos(a) * 24, 34 + Math.sin(-a) * 9]);
  }
  bubble.push([24, 20], [-4, 20], [-12, 12], [-10, 20], [-24, 20]);
  s.slab(bubble.reverse(), 16, 3, C.algodon, { front: C.papel, layer: 1 });
  // Three dots in the bubble.
  for (const x of [-10, 0, 10]) s.blob(x, 15.2, 31, 3, 1, 3, C.anil, { layer: 1.2, rand: r, jitter: 0.05 });
  // Three conjugated forms: prefix + root + ending.
  const rowsDef = [
    [C.mostaza, 6, C.anil, 15, C.bugambilia, 5],
    [C.jade, 5, C.anil, 15, C.bugambilia, 7],
    [C.cempasuchil, 7, C.anil, 15, C.salvia, 4],
  ];
  rowsDef.forEach((row, k) => {
    let x = -30 + k * 4;
    const y = -30 + k * 13, z = 0;
    for (let i = 0; i < row.length; i += 2) {
      const color = row[i], w = row[i + 1];
      s.box(x, y, z, w * 1.4, 8, 7, color, { top: mix(color, C.papel, 0.18), layer: 1.5 });
      x += w * 1.4 + 1.2;
    }
  });
  return s.svg({ yaw: -24, pitch: 30, title: "Conjugador" });
};

// ---------- Revista: an open magazine and a cup of pencils ----------
scenes.revista = () => {
  const r = rng(3), s = new Scene();
  s.box(-31, -20, 0, 62, 40, 0.8, C.terracota, { layer: 1 });               // cover
  s.box(-30, -19, 0.8, 29.4, 38, 1.6, C.hueso, { top: C.papel, layer: 1.2 });  // left pages
  s.box(0.6, -19, 0.8, 29.4, 38, 1.6, C.hueso, { top: C.papel, layer: 1.2 });  // right pages
  // Left page: picture + lines.
  s.box(-27, 1, 2.4, 23, 14, 0.4, C.turquesa, { layer: 2 });
  s.cone(-20, 6, 2.8, 4, 5, 3, C.pino, { layer: 2.1, phase: 0.3 });
  for (let i = 0; i < 5; i++) s.box(-27, -4 - i * 3, 2.4, 23 - (i % 2) * 6, 1.2, 0.3, C.lino, { layer: 2 });
  // Right page: title, picture, lines.
  s.box(3.5, 9, 2.4, 22, 5, 0.4, C.terracota, { layer: 2 });
  s.box(3.5, -6, 2.4, 10, 12, 0.4, C.mostaza, { layer: 2 });
  for (let i = 0; i < 5; i++) s.box(15.5, 4 - i * 2.6, 2.4, 10 - (i % 2) * 3, 1.1, 0.3, C.lino, { layer: 2 });
  for (let i = 0; i < 3; i++) s.box(3.5, -10 - i * 2.6, 2.4, 22 - i * 4, 1.1, 0.3, C.lino, { layer: 2 });
  // Cup of pencils.
  s.cylinder(44, 12, 0, 7, 13, 10, C.talavera, { top: C.anil, layer: 1.5 });
  [[41.5, 11, C.mostaza, 22], [46, 14.5, C.bugambilia, 19], [45, 9, C.jade, 24]].forEach(([x, y, c, h]) => {
    s.cylinder(x, y, 8, 1.5, h - 8, 6, c, { layer: 1.6 });
    s.cone(x, y, h, 1.5, 3.6, 6, C.arena, { layer: 1.7 });
  });
  return s.svg({ yaw: -22, pitch: 48, title: "Revista Mati Mati" });
};

// ---------- Instagram: a phone with a grid of posts and a heart ----------
scenes.instagram = () => {
  const r = rng(9), s = new Scene();
  s.box(-13, -24, 0, 26, 48, 2.4, C.tinta, { layer: 1 });
  s.box(-11.5, -21, 2.4, 23, 41, 0.3, C.hueso, { layer: 1.2 });
  const tiles = [C.terracota, C.turquesa, C.mostaza, C.jacaranda, C.nopal, C.bugambilia, C.anil, C.cempasuchil, C.jade];
  tiles.forEach((c, k) => {
    const i = k % 3, j = Math.floor(k / 3);
    s.box(-10.5 + i * 7.3, 7 - j * 7.3, 2.7, 6.6, 6.6, 0.5 + r() * 0.6, c, { layer: 2 });
  });
  s.box(-10.5, 15, 2.7, 4, 4, 0.5, C.terracota, { layer: 2 });          // avatar
  s.box(-5.5, 16.2, 2.7, 12, 1.4, 0.4, C.piedra, { layer: 2 });
  s.box(-10.5, -19.5, 2.7, 21, 3.5, 0.4, C.lino, { layer: 2 });
  // Heart, standing.
  const heart = [];
  for (let i = 0; i < 20; i++) {
    const t = (i / 20) * Math.PI * 2;
    heart.push([26 + 0.75 * 16 * Math.sin(t) ** 3, 22 + 0.75 * (13 * Math.cos(t) - 5 * Math.cos(2 * t) - 2 * Math.cos(3 * t) - Math.cos(4 * t))]);
  }
  s.slab(heart, 4, 5, C.sandia, { layer: 2.5 });
  return s.svg({ yaw: -30, pitch: 36, title: "@matidemati" });
};

const out = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "static", "img", "proyectos");
mkdirSync(out, { recursive: true });
for (const [id, make] of Object.entries(scenes)) {
  const svg = make();
  writeFileSync(join(out, `${id}.svg`), svg);
  console.log(`${id}.svg  ${(svg.length / 1024).toFixed(1)} KB`);
}
