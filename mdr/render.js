const fs=require('fs'); const {marked}=require('./node_modules/marked');
const [,,inp,out]=process.argv;
const html=marked.parse(fs.readFileSync(inp,'utf8'),{gfm:true});
fs.writeFileSync(out,`<!doctype html><meta charset=utf-8><style>body{font:14px/1.5 -apple-system,Segoe UI,Helvetica,Arial;max-width:900px;margin:24px auto;color:#1f2328}code{background:#eff1f3;padding:1px 4px;border-radius:4px;font-size:85%}pre{background:#f6f8fa;padding:12px;overflow:auto}table{border-collapse:collapse}td,th{border:1px solid #d0d7de;padding:4px 8px}img{max-width:100%}</style>${html}`);
