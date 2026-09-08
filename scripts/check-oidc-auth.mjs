import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'

const forbiddenEnv = /^(?:NPM_TOKEN|NODE_AUTH_TOKEN)$/i
for (const [name, value] of Object.entries(process.env)) {
  if (
    value &&
    (forbiddenEnv.test(name) || /^npm_config_.*(?:authtoken|_auth|_password|token)$/i.test(name))
  ) {
    throw new Error(
      'classic npm authentication environment is set; trusted publishing requires OIDC',
    )
  }
}

const configPaths = new Set()
const add = (path) => {
  if (path && path !== 'undefined' && path !== 'null') configPaths.add(path)
}
add('.npmrc')
add(process.env.NPM_CONFIG_USERCONFIG)
add(process.env.NPM_CONFIG_GLOBALCONFIG)
try {
  add(execFileSync('npm', ['config', 'get', 'userconfig'], { encoding: 'utf8' }).trim())
} catch {}
try {
  add(execFileSync('npm', ['config', 'get', 'globalconfig'], { encoding: 'utf8' }).trim())
} catch {}
for (const file of configPaths) {
  if (!existsSync(file)) continue
  const lines = readFileSync(file, 'utf8').split(/\r?\n/)
  if (
    lines.some((line) =>
      /^\s*(?:[^#;=]+:)?(?:_authToken|_auth|username|_password|email|token)\s*=\s*/i.test(line),
    )
  ) {
    throw new Error(
      'classic npm authentication was found in npm config; trusted publishing requires OIDC',
    )
  }
}
const version = execFileSync('npm', ['--version'], { encoding: 'utf8' })
  .trim()
  .split('.')
  .map(Number)
if (
  version[0] < 11 ||
  (version[0] === 11 && (version[1] < 5 || (version[1] === 5 && version[2] < 1)))
) {
  throw new Error(`npm ${version.join('.')} is below the trusted-publishing minimum 11.5.1`)
}
console.log(`verified npm ${version.join('.')}, sanitized config, and OIDC-only auth contract`)
