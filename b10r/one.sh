#!/usr/bin/env bash
# one.sh <svg> <png> [scale] : render one svg to png
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [s,o,sc]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:+(sc||2)});
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block;padding:4px">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();' "$@"
