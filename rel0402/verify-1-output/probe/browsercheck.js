// Compare hit_test results with Chromium's elementFromPoint on the same SVG.
// usage: node browsercheck.js <dir> <name>...
const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

(async () => {
  const [dir, ...names] = process.argv.slice(2);
  const b = await chromium.launch();
  const page = await b.newPage({ viewport: { width: 4000, height: 4000 } });
  const totals = {};
  for (const name of names) {
    const svg = fs.readFileSync(path.join(dir, name + ".svg"), "utf8");
    const rows = fs.readFileSync(path.join(dir, name + ".hits.tsv"), "utf8").trim().split("\n").map((l) => l.split("\t"));
    await page.setContent(`<html><body style="margin:0;padding:0">${svg}</body></html>`);
    const kind = svg.includes('aria-label="Mind map"') ? "mind" : svg.includes('aria-label="Sankey diagram"') ? "sankey" : "pie";
    const pts = rows.map((r) => [parseFloat(r[0]), parseFloat(r[1])]);
    const got = await page.evaluate(({ pts, kind }) => {
      const svg = document.querySelector("svg");
      const r0 = svg.getBoundingClientRect();
      const all = Array.from(svg.querySelectorAll("*"));
      const idx = new Map();
      if (kind === "sankey") {
        let li = 0, ni = 0;
        for (const e of all) {
          if (e.tagName === "path" && e.getAttribute("fill-opacity") === "0.45") idx.set(e, "Link(" + li++ + ")");
          else if (e.tagName === "rect" && e.querySelector("title")) idx.set(e, "Node(" + ni++ + ")");
        }
      } else if (kind === "mind") {
        let li = 0, ni = 0, links = true;
        for (const e of all) {
          if (e.closest("defs") || e.tagName === "title" || e.tagName === "svg") continue;
          if (e.tagName === "rect" && e.parentNode === svg && idx.size === 0 && li === 0) { idx.set(e, "BG"); continue; }
          if (links && e.tagName === "path" && e.getAttribute("stroke-linecap") === "round" && e.getAttribute("fill") === "none") { idx.set(e, "Link(" + li++ + ")"); continue; }
          if (e.tagName === "g") continue;
          links = false;
          if (e.tagName === "tspan") { idx.set(e, "NodeText(" + ni + ")"); continue; }
          if (e.tagName === "text") { idx.set(e, "NodeText(" + ni + ")"); ni++; continue; }
          idx.set(e, (e.tagName === "path" && e.getAttribute("fill") === "none" ? "NodeIcon(" : "Node(") + ni + ")");
        }
      }
      return pts.map(([x, y]) => {
        const e = document.elementFromPoint(r0.left + x, r0.top + y);
        if (!e || e === svg || e.tagName === "HTML" || e.tagName === "BODY") return "BG";
        if (kind === "pie") {
          const t = e.tagName;
          if (t === "text" || t === "tspan") return "TEXT";
          if (t === "line") return "SEP";
          if (t === "polyline") return "LEADER";
          const fill = e.getAttribute("fill") || "";
          if (t === "rect" && e.getAttribute("rx") === "2") return "LEG|" + fill;
          if (t === "rect") return "BG";
          if ((t === "path" || t === "circle") && fill !== "none") return "SLICE|" + fill;
          return "OTHER:" + t;
        }
        if (e.tagName === "text" || e.tagName === "tspan") return idx.get(e) || "TEXT";
        return idx.get(e) || (e.tagName === "rect" ? "BG" : "OTHER:" + e.tagName);
      });
    }, { pts, kind });
    // compare
    const t = (totals[kind] = totals[kind] || { pts: 0, agree: 0, text: 0, mism: {}, ex: [] });
    rows.forEach((r, i) => {
      const rust = r[2], br = got[i];
      t.pts++;
      let ok;
      if (kind === "pie") {
        if (br === "TEXT" || br === "SEP") { t.text++; return; }
        if (br === "BG") ok = rust === "None";
        else if (br === "LEADER") ok = rust.startsWith("Label");
        else if (br.startsWith("SLICE|")) {
          const f = br.slice(6);
          ok = rust.startsWith("Slice(") && (f === rust.split("|")[1] || (f.startsWith("url(") && rust.split("|")[2] === "true"));
          if (!ok && rust.startsWith("Label")) { t.text++; return; } // label text box over a wedge
        } else if (br.startsWith("LEG|")) {
          const f = br.slice(4);
          ok = rust.startsWith("Legend(") && (f === rust.split("|")[1] || (f.startsWith("url(") && rust.split("|")[2] === "true"));
        } else ok = false;
      } else if (kind === "sankey") {
        if (br === "TEXT") { t.text++; return; }
        if (br === "BG") ok = rust === "None" || (rust.startsWith("Node(")); // label box beyond glyphs
        else ok = br === rust || (br.startsWith("Link") && rust.startsWith("Node(")); // label box over ribbon
        if (ok && br !== rust && br !== "BG") { t.text++; return; }
        if (ok && br === "BG" && rust !== "None") { t.text++; return; }
      } else {
        if (br.startsWith("NodeText(")) { const n = br.slice(9, -1); ok = rust === "Node(" + n + ")"; if (!ok) { t.text++; return; } }
        else if (br.startsWith("NodeIcon(")) ok = rust === "Node(" + br.slice(9, -1) + ")";
        else if (br === "BG") ok = rust === "None";
        else ok = br === rust;
      }
      if (ok) t.agree++;
      else {
        const key = br.replace(/\d+/g, "N") + " vs " + rust.replace(/\d+/g, "N").split("|")[0];
        t.mism[key] = (t.mism[key] || 0) + 1;
        if (t.ex.length < 12) t.ex.push(`${name} (${r[0]},${r[1]}) browser=${br} rust=${rust}`);
      }
    });
  }
  await b.close();
  console.log(JSON.stringify(totals, null, 1));
})();
