import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(new URL('..', import.meta.url).pathname)
const packDir = resolve(root, process.env.PACK_DIR ?? '.release/pack')
rmSync(packDir, { recursive: true, force: true })
mkdirSync(packDir, { recursive: true })
const source = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8'))
const raw = execFileSync(
  'npm',
  ['pack', '--ignore-scripts', '--json', '--pack-destination', packDir],
  { cwd: root, encoding: 'utf8' },
)
const packed = JSON.parse(raw)
if (!Array.isArray(packed) || packed.length !== 1 || typeof packed[0]?.filename !== 'string')
  throw new Error('npm pack did not produce exactly one archive')
const archive = resolve(packDir, packed[0].filename)
const archives = readdirSync(packDir).filter((name) => name.endsWith('.tgz'))
if (!existsSync(archive) || archives.length !== 1 || archives[0] !== packed[0].filename)
  throw new Error('pack directory must contain exactly the captured tarball')
const packedManifest = JSON.parse(
  execFileSync('tar', ['-xOzf', archive, 'package/package.json'], { encoding: 'utf8' }),
)
for (const field of [
  'name',
  'version',
  'main',
  'module',
  'types',
  'exports',
  'dependencies',
  'optionalDependencies',
]) {
  if (JSON.stringify(packedManifest[field]) !== JSON.stringify(source[field]))
    throw new Error(`manifest field ${field} changed in tarball`)
}
if (JSON.stringify(packedManifest).includes('workspace:'))
  throw new Error('workspace dependency leaked into tarball')
const entries = execFileSync('tar', ['-tzf', archive], { encoding: 'utf8' })
  .trim()
  .split('\n')
  .filter(Boolean)
const expected = [
  'package/package.json',
  'package/README.md',
  'package/LICENSE',
  'package/dist/index.js',
  'package/dist/index.d.ts',
]
for (const file of expected)
  if (!entries.includes(file)) throw new Error(`tarball is missing ${file}`)
const forbidden = entries.filter((file) =>
  /^package\/(?:src|tests|scripts|node_modules|\.github)(?:\/|$)/.test(file),
)
if (forbidden.length)
  throw new Error(`disallowed files leaked into tarball: ${forbidden.join(', ')}`)
if (process.env.GITHUB_ENV)
  writeFileSync(process.env.GITHUB_ENV, `AIHU_PACK_PATH=${archive}\n`, { flag: 'a' })
console.log(
  `verified ${archive}: ${packedManifest.name}@${packedManifest.version} (${entries.length} files)`,
)
