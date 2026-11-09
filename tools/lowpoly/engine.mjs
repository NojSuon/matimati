// A tiny low-poly renderer: 3D shapes → flat-shaded facets → SVG.
// No dependencies. World axes: x right, y back, z up. Orthographic camera.

// ---------- Vectors ----------
export const v = (x, y, z) => ({ x, y, z });
const sub = (a, b) => v(a.x - b.x, a.y - b.y, a.z - b.z);
const add = (a, b) => v(a.x + b.x, a.y + b.y, a.z + b.z);
const mul = (a, k) => v(a.x * k, a.y * k, a.z * k);
const dot = (a, b) => a.x * b.x + a.y * b.y + a.z * b.z;
const cross = (a, b) => v(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x);
const len = (a) => Math.hypot(a.x, a.y, a.z);
const norm = (a) => mul(a, 1 / (len(a) || 1));
const centroid = (pts) => mul(pts.reduce(add, v(0, 0, 0)), 1 / pts.length);

// ---------- Seeded randomness (same art on every run) ----------
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---------- Colours ----------
const hex = (c) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
const toHex = (a) => "#" + a.map((n) => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0")).join("");
export const mix = (a, b, t) => toHex(hex(a).map((x, i) => x + (hex(b)[i] - x) * t));

// ---------- Scene ----------
export class Scene {
  constructor() {
    this.faces = [];
  }
  /** Adds a face. `center` (the solid's centre) orients its normal outward;
      open surfaces pass `up: true` to face the sky instead. */
  face(pts, color, { layer = 1, center = null, up = false, flat = false } = {}) {
    this.faces.push({ pts, color, layer, center, up, flat });
  }

  // Axis-aligned box from (x, y, z) with size (w, d, h). `top` colours the lid.
  box(x, y, z, w, d, h, color, { top = color, layer = 1, rot = 0, cx = null, cy = null } = {}) {
    const ox = cx ?? x + w / 2, oy = cy ?? y + d / 2;
    const r = (p) => rotZ(p, rot, ox, oy);
    const P = (a, b, c) => r(v(x + a * w, y + b * d, z + c * h));
    const c = r(v(x + w / 2, y + d / 2, z + h / 2));
    const quads = [
      [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1], top],
      [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0], color],
      [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1], color],
      [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1], color],
      [[1, 1, 0], [0, 1, 0], [0, 1, 1], [1, 1, 1], color],
      [[0, 1, 0], [0, 0, 0], [0, 0, 1], [0, 1, 1], color],
    ];
    for (const q of quads) this.face(q.slice(0, 4).map((p) => P(...p)), q[4], { layer, center: c });
    return this;
  }

  // Upright prism over a 2D outline [[x, y], …] (counter-clockwise), from z to z + h.
  prism(outline, z, h, color, { top = color, layer = 1 } = {}) {
    const lo = outline.map(([x, y]) => v(x, y, z));
    const hi = outline.map(([x, y]) => v(x, y, z + h));
    const c = centroid([...lo, ...hi]);
    this.face(hi, top, { layer, center: c });
    this.face([...lo].reverse(), color, { layer, center: c });
    for (let i = 0; i < outline.length; i++) {
      const j = (i + 1) % outline.length;
      this.face([lo[i], lo[j], hi[j], hi[i]], color, { layer, center: c });
    }
    return this;
  }

  // Prism lying in the x–z plane (an outline [[x, z], …]) with thickness along y.
  slab(outline, y, d, color, { front = color, layer = 1 } = {}) {
    const a = outline.map(([x, z]) => v(x, y, z));
    const b = outline.map(([x, z]) => v(x, y + d, z));
    const c = centroid([...a, ...b]);
    this.face(a, front, { layer, center: c });
    this.face([...b].reverse(), color, { layer, center: c });
    for (let i = 0; i < outline.length; i++) {
      const j = (i + 1) % outline.length;
      this.face([a[i], a[j], b[j], b[i]], color, { layer, center: c });
    }
    return this;
  }

  cylinder(cx, cy, z, r, h, n, color, opts = {}) {
    const outline = ring(cx, cy, r, n, opts.phase ?? 0, opts.sx ?? 1, opts.sy ?? 1);
    return this.prism(outline, z, h, color, opts);
  }

  cone(cx, cy, z, r, h, n, color, { layer = 1, phase = 0, apex = null, sx = 1, sy = 1 } = {}) {
    const base = ring(cx, cy, r, n, phase, sx, sy).map(([x, y]) => v(x, y, z));
    const tip = apex ?? v(cx, cy, z + h);
    const c = centroid([...base, tip]);
    for (let i = 0; i < n; i++) this.face([base[i], base[(i + 1) % n], tip], color, { layer, center: c });
    this.face([...base].reverse(), color, { layer, center: c });
    return this;
  }

  // Faceted ellipsoid (a subdivided octahedron), slightly jittered.
  blob(cx, cy, cz, rx, ry, rz, color, { layer = 1, detail = 1, jitter = 0.08, rand = Math.random, rot = 0 } = {}) {
    let tris = [
      [v(1, 0, 0), v(0, 1, 0), v(0, 0, 1)], [v(0, 1, 0), v(-1, 0, 0), v(0, 0, 1)],
      [v(-1, 0, 0), v(0, -1, 0), v(0, 0, 1)], [v(0, -1, 0), v(1, 0, 0), v(0, 0, 1)],
      [v(0, 1, 0), v(1, 0, 0), v(0, 0, -1)], [v(-1, 0, 0), v(0, 1, 0), v(0, 0, -1)],
      [v(0, -1, 0), v(-1, 0, 0), v(0, 0, -1)], [v(1, 0, 0), v(0, -1, 0), v(0, 0, -1)],
    ];
    for (let d = 0; d < detail; d++) {
      tris = tris.flatMap(([a, b, c]) => {
        const ab = norm(add(a, b)), bc = norm(add(b, c)), ca = norm(add(c, a));
        return [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]];
      });
    }
    const cache = new Map();
    const place = (p) => {
      const key = `${p.x.toFixed(4)},${p.y.toFixed(4)},${p.z.toFixed(4)}`;
      if (!cache.has(key)) {
        const k = 1 + (rand() - 0.5) * 2 * jitter;
        cache.set(key, rotZ(v(cx + p.x * rx * k, cy + p.y * ry * k, cz + p.z * rz * k), rot, cx, cy));
      }
      return cache.get(key);
    };
    const c = v(cx, cy, cz);
    for (const t of tris) this.face(t.map(place), color, { layer, center: c });
    return this;
  }

  // A rippled surface (water, ground): a grid of triangles facing up.
  surface(x, y, w, d, nx, ny, heightAt, colorAt, { layer = 0 } = {}) {
    const P = (i, j) => {
      const px = x + (w * i) / nx, py = y + (d * j) / ny;
      return v(px, py, heightAt(px, py));
    };
    for (let i = 0; i < nx; i++) {
      for (let j = 0; j < ny; j++) {
        const a = P(i, j), b = P(i + 1, j), c = P(i + 1, j + 1), e = P(i, j + 1);
        const t1 = (i + j) % 2 ? [a, b, c] : [a, b, e];
        const t2 = (i + j) % 2 ? [a, c, e] : [b, c, e];
        for (const t of [t1, t2]) this.face(t, colorAt(centroid(t)), { layer, up: true });
      }
    }
    return this;
  }

  /**
   * Renders to SVG. camera: yaw (degrees around z), pitch (degrees looking
   * down). light: direction toward the light in screen space.
   */
  svg({ yaw = -35, pitch = 32, width = 960, height = 600, pad = 24, light = v(-0.45, -0.6, 1), shadeMin = 0.6, title = "" } = {}) {
    const cy = Math.cos((yaw * Math.PI) / 180), sy = Math.sin((yaw * Math.PI) / 180);
    const cp = Math.cos((pitch * Math.PI) / 180), sp = Math.sin((pitch * Math.PI) / 180);
    const view = (p) => {
      const x = p.x * cy - p.y * sy, y = p.x * sy + p.y * cy;
      return { sx: x, sy: -(p.z * cp + y * sp), depth: y * cp - p.z * sp, r: v(x, y, p.z) };
    };
    const viewDir = v(0, cp, -sp);
    const L = norm(light);
    const out = [];
    for (const f of this.faces) {
      const pv = f.pts.map(view);
      const rp = pv.map((q) => q.r);
      let n = norm(cross(sub(rp[1], rp[0]), sub(rp[2], rp[0])));
      if (f.up) {
        if (n.z < 0) n = mul(n, -1);
      } else if (f.center) {
        const c = view(f.center).r;
        if (dot(n, sub(centroid(rp), c)) < 0) n = mul(n, -1);
        if (dot(n, viewDir) >= 0) continue; // back face
      }
      const lit = Math.max(0, dot(n, L));
      const k = shadeMin + (1 - shadeMin) * lit;
      let color = mix("#000000", f.color, Math.min(1, k + 0.12));
      if (lit > 0.92) color = mix(color, "#FFF9F5", (lit - 0.92) * 1.6);
      out.push({ layer: f.layer, depth: pv.reduce((s, q) => s + q.depth, 0) / pv.length, pts: pv, color });
    }
    out.sort((a, b) => a.layer - b.layer || b.depth - a.depth);

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const f of out) for (const q of f.pts) {
      minX = Math.min(minX, q.sx); maxX = Math.max(maxX, q.sx);
      minY = Math.min(minY, q.sy); maxY = Math.max(maxY, q.sy);
    }
    const s = Math.min((width - 2 * pad) / (maxX - minX), (height - 2 * pad) / (maxY - minY));
    const ox = (width - (maxX - minX) * s) / 2 - minX * s;
    const oy = (height - (maxY - minY) * s) / 2 - minY * s;
    const fmt = (n) => (Math.round(n * 10) / 10).toString();

    // Group faces by colour to keep the file small.
    const paths = out.map((f) => {
      const d = "M" + f.pts.map((q) => `${fmt(q.sx * s + ox)} ${fmt(q.sy * s + oy)}`).join("L") + "Z";
      return `<path fill="${f.color}" stroke="${f.color}" d="${d}"/>`;
    });
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-label="${title}">` +
      `<g stroke-width="0.8" stroke-linejoin="round">${paths.join("")}</g></svg>\n`;
  }
}

export function ring(cx, cy, r, n, phase = 0, sx = 1, sy = 1) {
  return Array.from({ length: n }, (_, i) => {
    const a = phase + (i / n) * Math.PI * 2;
    return [cx + Math.cos(a) * r * sx, cy + Math.sin(a) * r * sy];
  });
}

export function rotZ(p, deg, ox = 0, oy = 0) {
  if (!deg) return p;
  const a = (deg * Math.PI) / 180, c = Math.cos(a), s = Math.sin(a);
  const x = p.x - ox, y = p.y - oy;
  return v(ox + x * c - y * s, oy + x * s + y * c, p.z);
}
