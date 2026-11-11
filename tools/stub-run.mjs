// Dry run of a workflow script with stubbed agents: prints each agent label and the sha1 of its prompt.
// usage: node stub-run.mjs <script.js> '<args json>' [fail-rounds]
import fs from 'fs'; import crypto from 'crypto'
const [,, file, argsJson = '{}', failRounds = '0'] = process.argv
let src = fs.readFileSync(file, 'utf8').replace(/^export const meta/m, 'const meta')
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor
let fails = +failRounds
const agent = async (prompt, opts = {}) => {
  const h = crypto.createHash('sha1').update(prompt).digest('hex').slice(0, 12)
  console.log(`${(opts.label || '?').padEnd(34)} ${h} len=${prompt.length}`)
  if (process.env.SHOW && (opts.label || '').startsWith(process.env.SHOW)) console.log(prompt)
  const l = opts.label || ''
  if (l.startsWith('pre:')) return { sha: 'abc123' }
  if (l.startsWith('verify:')) { const v = fails-- > 0 ? 'fail' : 'pass'; return { verdict: v, guard_green: true, perf_ok: true, verified_fixes: [], problems: v === 'fail' ? ['p1'] : [], unfixed_items: [], notes: '' } }
  if (l.startsWith('impl:') || l.startsWith('fix:')) return { summary: 's', commits: [], items_done: [], items_skipped: [], guard_green: true, perf_ok: true }
  return 'ok'
}
const fn = new AsyncFunction('agent', 'log', 'phase', 'args', 'parallel', 'pipeline', 'budget', src)
const out = await fn(agent, m => console.log('  log:', m), t => console.log('phase:', t), JSON.parse(argsJson), null, null, { total: null })
console.log(JSON.stringify(out).slice(0, 400))
