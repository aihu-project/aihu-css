import { execFileSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const archive = process.argv[2]
if (!archive) throw new Error('usage: node scripts/consumer-smoke.mjs /absolute/path/package.tgz')
const dir = mkdtempSync(join(tmpdir(), 'aihu-css-consumer-'))
execFileSync('npm', ['init', '-y'], { cwd: dir, stdio: 'ignore' })
execFileSync(
  'npm',
  ['install', '--ignore-scripts', '--no-package-lock', '--no-audit', '--no-fund', resolve(archive)],
  { cwd: dir, stdio: 'inherit' },
)
execFileSync(
  process.execPath,
  [
    '--input-type=module',
    '-e',
    "const mod = await import('@aihu/css-engine'); if (typeof mod !== 'object') throw new Error('CSS engine import failed')",
  ],
  { cwd: dir, stdio: 'inherit' },
)
console.log(`isolated consumer passed against ${archive}`)
