#!/usr/bin/env bash
# tri.sh <out.png> <scale> <a.svg> <b.svg> <c.svg>: three SVGs side by side with labels
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [o,sc,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:+sc});
const parts=f.map(x=>`<div style="background:#fff;padding:4px"><div style="font:12px sans-serif">${x.split("/").slice(-2).join("/")}</div>${fs.readFileSync(x,"utf8")}</div>`);
await p.setContent(`<html><body style="margin:0;background:#888"><div id=w style="display:inline-flex;gap:6px;align-items:flex-start">${parts.join("")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();' "$@"
