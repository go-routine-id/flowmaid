// Package A private helper: loads the REAL flowmaid render public/svg/hero.svg at runtime,
// splits it into its parts (subgraph frames, edges, nodes, label chips, frame titles) and
// measures what the cascade needs (node boxes via getBBox, edge lengths via getTotalLength)
// in an off-screen mount. Deterministic: same file + same browser -> same numbers; the
// render waits for it with delayRender / continueRender.
//
// hero.svg structure (flowmaid 0.40): <defs><marker id="arrow">; a subgraph frame
// <rect> (not followed by <text>); edges = <path fill="none" marker-end> with
// <title>A → B[: label]</title>; nodes = <g><title>Name</title>...</g>; label chips and
// the subgraph title = <rect> immediately followed by <text>.
import { useEffect, useState } from 'react';
import { cancelRender, continueRender, delayRender, staticFile } from 'remotion';

export type Box = { x: number; y: number; w: number; h: number };
export type HeroEdge = { from: string; to: string; label?: string; d: string; stroke: string; strokeWidth: number; marker: string | null; len: number };
export type HeroNode = { name: string; html: string; box: Box };
export type HeroChip = { label: string; html: string; box: Box };
export type HeroFrame = { box: Box; rx: number; fill: string; stroke: string; strokeWidth: number };
export type Hero = {
  w: number; h: number; fontFamily: string; fontSize: string; defs: string;
  frames: HeroFrame[]; edges: HeroEdge[]; nodes: HeroNode[];
  chips: HeroChip[]; // edge label chips (label matches an edge label)
  titles: HeroChip[]; // other rect+text pairs (subgraph titles)
  ink: Box; // union of everything drawn (for the breath origin)
};

const ID_PREFIX = 'fmA-hero-'; // marker ids renamed so they can never clash with another inlined SVG

let cache: Hero | null = null;
let pending: Promise<Hero> | null = null;

const num = (el: Element, a: string) => parseFloat(el.getAttribute(a) ?? '0');
const rectBox = (r: Element): Box => ({ x: num(r, 'x'), y: num(r, 'y'), w: num(r, 'width'), h: num(r, 'height') });
const union = (a: Box | null, b: Box): Box => {
  if (!a) return { ...b };
  const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
  return { x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y };
};
const renameIds = (s: string) => s.replace(/id="([^"]+)"/g, `id="${ID_PREFIX}$1"`).replace(/url\(#([^)]+)\)/g, `url(#${ID_PREFIX}$1)`);

const parse = (text: string): Hero => {
  const doc = new DOMParser().parseFromString(text, 'image/svg+xml');
  const src = doc.documentElement;
  // off-screen live mount so getBBox / getTotalLength work
  const host = document.createElement('div');
  host.style.cssText = 'position:absolute;left:-30000px;top:0;visibility:hidden;pointer-events:none;';
  const svg = document.importNode(src, true) as unknown as SVGSVGElement;
  host.appendChild(svg);
  document.body.appendChild(host);
  try {
    const out: Hero = {
      w: num(svg, 'width'), h: num(svg, 'height'),
      fontFamily: svg.getAttribute('font-family') ?? 'sans-serif', fontSize: svg.getAttribute('font-size') ?? '14',
      defs: '', frames: [], edges: [], nodes: [], chips: [], titles: [], ink: { x: 0, y: 0, w: 0, h: 0 },
    };
    let ink: Box | null = null;
    const kids = Array.from(svg.children);
    const pairs: { rect: Element; text: Element }[] = [];
    for (let i = 0; i < kids.length; i++) {
      const el = kids[i];
      const tag = el.tagName.toLowerCase();
      if (tag === 'defs') {
        out.defs = renameIds(el.innerHTML);
      } else if (tag === 'g') {
        const title = el.querySelector('title')?.textContent ?? '';
        const b = (el as SVGGraphicsElement).getBBox();
        const box = { x: b.x, y: b.y, w: b.width, h: b.height };
        out.nodes.push({ name: title, html: renameIds(el.innerHTML), box });
        ink = union(ink, box);
      } else if (tag === 'path' && el.getAttribute('fill') === 'none' && el.getAttribute('marker-end')) {
        const t = el.querySelector('title')?.textContent ?? '';
        const m = /^\s*(\S+)\s*→\s*([^:]+?)\s*(?::\s*(.+))?$/.exec(t);
        if (!m) throw new Error(`hero.svg: unparsed edge title "${t}"`);
        const me = el.getAttribute('marker-end');
        out.edges.push({
          from: m[1], to: m[2], label: m[3]?.trim(), d: el.getAttribute('d') ?? '',
          stroke: el.getAttribute('stroke') ?? '#000', strokeWidth: num(el, 'stroke-width') || 1,
          marker: me ? renameIds(me) : null, len: (el as SVGGeometryElement).getTotalLength(),
        });
        const b = (el as SVGGraphicsElement).getBBox();
        ink = union(ink, { x: b.x, y: b.y, w: b.width, h: b.height });
      } else if (tag === 'rect') {
        const next = kids[i + 1];
        if (next && next.tagName.toLowerCase() === 'text') {
          pairs.push({ rect: el, text: next });
          i++;
        } else {
          const box = rectBox(el);
          out.frames.push({
            box, rx: num(el, 'rx'), fill: el.getAttribute('fill') ?? 'none', stroke: el.getAttribute('stroke') ?? 'none',
            strokeWidth: num(el, 'stroke-width') || 1,
          });
          ink = union(ink, box);
        }
      }
    }
    const edgeLabels = new Set(out.edges.map((e) => e.label).filter(Boolean));
    for (const { rect, text } of pairs) {
      const label = text.textContent ?? '';
      const chip: HeroChip = { label, html: rect.outerHTML + text.outerHTML, box: rectBox(rect) };
      (edgeLabels.has(label) ? out.chips : out.titles).push(chip);
      ink = union(ink, chip.box);
    }
    out.ink = ink ?? { x: 0, y: 0, w: out.w, h: out.h };
    return out;
  } finally {
    host.remove();
  }
};

const load = () => {
  if (!pending) pending = fetch(staticFile('svg/hero.svg')).then((r) => r.text()).then(parse);
  return pending;
};

/** The parsed + measured hero.svg (null on the very first render, before it has loaded). */
export const useHeroSvg = (): Hero | null => {
  const [hero, setHero] = useState<Hero | null>(cache);
  const [handle] = useState(() => (cache ? null : delayRender('package A: hero.svg')));
  useEffect(() => {
    if (handle === null) return;
    load()
      .then((h) => {
        cache = h;
        setHero(h);
        continueRender(handle);
      })
      .catch((err) => cancelRender(err));
  }, [handle]);
  return hero;
};

/** rounded-rect perimeter (stroke dash length of an SVG <rect rx>). */
export const rectPerimeter = (b: Box, rx: number) => {
  const r = Math.min(rx, b.w / 2, b.h / 2);
  return 2 * (b.w + b.h) - 8 * r + 2 * Math.PI * r;
};
