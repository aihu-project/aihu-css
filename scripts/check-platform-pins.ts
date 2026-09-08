import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

type PackageJson = {
  name?: unknown
  version?: unknown
  optionalDependencies?: Record<string, unknown>
}

const root = resolve(fileURLToPath(new URL('..', import.meta.url)))

const readJson = (path: string): PackageJson =>
  JSON.parse(readFileSync(path, 'utf8')) as PackageJson

/**
 * Verify the standalone host/native release invariant.
 *
 * The host and platform packages intentionally have separate version streams:
 * `@aihu/css-engine` is 0.6.x while its native binaries are 0.1.x. The
 * native stream is nevertheless exact: every host optionalDependency pin must
 * be a concrete version, every platform manifest must carry that exact pin,
 * and all four platform packages must move together.
 */
export function checkPlatformPins(packageRoot = root): void {
  const host = readJson(resolve(packageRoot, 'package.json'))
  const hostName = typeof host.name === 'string' ? host.name : 'host package'
  const optional = host.optionalDependencies ?? {}
  const nativeNames = Object.keys(optional)
    .filter((name) => name.startsWith('@aihu/css-engine-'))
    .sort()

  const expectedNames = [
    '@aihu/css-engine-darwin-arm64',
    '@aihu/css-engine-darwin-x64',
    '@aihu/css-engine-linux-x64-gnu',
    '@aihu/css-engine-win32-x64-msvc',
  ]

  if (JSON.stringify(nativeNames) !== JSON.stringify(expectedNames)) {
    throw new Error(
      `${hostName} must pin exactly ${expectedNames.join(', ')}; found ${nativeNames.join(', ') || '(none)'}`,
    )
  }

  const versions = new Map<string, string>()
  for (const name of expectedNames) {
    const pin = optional[name]
    if (typeof pin !== 'string' || !/^\d+\.\d+\.\d+$/.test(pin)) {
      throw new Error(`${name} must use an exact stable version pin; found ${String(pin)}`)
    }

    const directory = name.slice('@aihu/css-engine-'.length)
    const manifest = readJson(resolve(packageRoot, 'npm', directory, 'package.json'))
    if (manifest.name !== name || manifest.version !== pin) {
      throw new Error(
        `${name} manifest must be ${name}@${pin}; found ${String(manifest.name)}@${String(manifest.version)}`,
      )
    }
    versions.set(name, pin)
  }

  const uniqueVersions = [...new Set(versions.values())]
  if (uniqueVersions.length !== 1) {
    throw new Error(
      `native platform packages must share one version; found ${[...versions.entries()]
        .map(([name, version]) => `${name}@${version}`)
        .join(', ')}`,
    )
  }
}

if (import.meta.main) {
  checkPlatformPins()
  console.log('✓ CSS native package pins are exact and synchronized')
}
