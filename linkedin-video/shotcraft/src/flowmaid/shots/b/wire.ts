// Shot 4 `types`: load the real flowmaid SVG renders (public/svg) and derive their BLUEPRINT
// ("wire") version from the very same markup — fills off, every shape stroked in --muted 1 px,
// text hidden, pathLength="1" injected so each shape draws on with stroke-dashoffset.
// Because the wire layer and the materialised layer are the same SVG placed in the same box,
// they align by construction (the wireframe-draw-on card's main pitfall).
//
// Per-frame cost is one CSS custom property (--t, 0..1) on the wrapper: every shape carries a
// precomputed calc() for its own staggered dash offset, so the markup is parsed once.
import { useEffect, useState } from 'react';
import { cancelRender, continueRender, delayRender, staticFile } from 'remotion';

const cache = new Map<string, Promise<string>>();
const load = (file: string): Promise<string> => {
  let p = cache.get(file);
  if (!p) {
    p = fetch(staticFile(file)).then((r) => {
      if (!r.ok) throw new Error(`types: cannot load ${file} (${r.status})`);
      return r.text();
    });
    cache.set(file, p);
  }
  return p;
};

/** markup of every file (in order), or null while loading (the render waits via delayRender). */
export const useSvgTexts = (files: readonly string[]): string[] | null => {
  const [texts, setTexts] = useState<string[] | null>(null);
  const [handle] = useState(() => delayRender('types: svg markup'));
  useEffect(() => {
    let alive = true;
    Promise.all(files.map(load))
      .then((t) => {
        if (alive) setTexts(t);
        continueRender(handle);
      })
      .catch((err) => cancelRender(err));
    return () => {
      alive = false;
    };
  }, [files, handle]);
  return texts;
};

export const viewBoxOf = (text: string): { w: number; h: number } => {
  const m = /viewBox="\s*([-\d.]+)[\s,]+([-\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)\s*"/.exec(text);
  if (!m) throw new Error('types: svg without viewBox');
  return { w: Number(m[3]), h: Number(m[4]) };
};

const SHAPES = 'path, rect, circle, ellipse, line, polyline, polygon';
const DASH = 1.002; // a hair over pathLength 1 so a finished stroke closes with no sliver

/**
 * Wire markup of one SVG. `strokeUser` = stroke width in USER units (1 px on screen = 1 / scale).
 * Shape j of n starts at t = lead·j/(n−1) and draws over `span` of the cell's 0..1 progress, in
 * document order (flowmaid paints frames, then edges, then nodes: it reads as being constructed).
 */
export const wireMarkup = (text: string, color: string, strokeUser: number, lead = 0.4, span = 0.6): string => {
  const doc = new DOMParser().parseFromString(text, 'image/svg+xml');
  const svg = doc.documentElement;
  svg.querySelectorAll('defs, title, desc, text, style').forEach((el) => el.remove());
  svg.querySelectorAll('[opacity], [fill-opacity], [stroke-opacity]').forEach((el) => {
    el.removeAttribute('opacity');
    el.removeAttribute('fill-opacity');
    el.removeAttribute('stroke-opacity');
  });
  const shapes = Array.from(svg.querySelectorAll(SHAPES));
  const n = shapes.length;
  const k = (DASH / span).toFixed(5);
  shapes.forEach((el, j) => {
    const d = (n > 1 ? (lead * j) / (n - 1) : 0).toFixed(5);
    ['marker-start', 'marker-mid', 'marker-end', 'filter', 'mask', 'clip-path', 'stroke-dasharray', 'stroke-linecap'].forEach((a) => el.removeAttribute(a));
    el.setAttribute('fill', 'none');
    el.setAttribute('stroke', color);
    el.setAttribute('stroke-width', strokeUser.toFixed(4));
    el.setAttribute('stroke-linejoin', 'miter');
    el.setAttribute('pathLength', '1');
    el.setAttribute(
      'style',
      `stroke-dasharray:${DASH} ${DASH};stroke-dashoffset:clamp(0, calc(${DASH} - (var(--t) - ${d}) * ${k}), ${DASH})`,
    );
  });
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  svg.removeAttribute('role');
  svg.setAttribute('aria-hidden', 'true');
  return new XMLSerializer().serializeToString(svg);
};
