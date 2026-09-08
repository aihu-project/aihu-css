import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

const manifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url)))
const spec = `${manifest.name}@${manifest.version}`
const result = spawnSync(
  'npm',
  ['view', spec, 'version', '--json', '--registry=https://registry.npmjs.org'],
  { encoding: 'utf8' },
)
if (result.status === 0)
  throw new Error(`${spec} already exists on npm; refusing to publish over it`)
const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`
const codes = [...output.matchAll(/(?:npm )?error code (E\d+)/gi)].map((match) =>
  match[1].toUpperCase(),
)
if (
  !/E404|No match found for version/i.test(output) ||
  codes.some((code) => code !== 'E404') ||
  /ECONN|ETIMEDOUT|ENETUNREACH|EAI_AGAIN/i.test(output)
) {
  throw new Error(`npm absence check did not fail closed with E404 for ${spec}`)
}
console.log(`confirmed E404-only absence for ${spec}`)
