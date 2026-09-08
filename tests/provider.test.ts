import { describe, expect, it, vi } from 'vitest'
import { aihuCssProvider, createAihuCssProvider } from '../src/index.ts'

describe('Aihu CSS provider contract', () => {
  it('adapts a compileSfc-compatible function for light-DOM compiler calls', () => {
    const compile = vi.fn(() => '.button[data-a="abc123"] { color: red; }')
    const provider = createAihuCssProvider(compile)

    const result = provider({
      source: '@template { <button class="button">Save</button> }',
      id: '/src/Button.aihu',
      shadowMode: 'light',
      target: 'client',
      lightScopeId: 'abc123',
    })

    expect(result).toContain('[data-a="abc123"]')
    expect(compile).toHaveBeenCalledOnce()
    expect(compile).toHaveBeenCalledWith(
      '@template { <button class="button">Save</button> }',
      '/src/Button.aihu',
      'abc123',
    )
  })

  it('does not pass a light scope into shadow-DOM compilation', () => {
    const compile = vi.fn(() => ':host { color: red; }')
    const provider = createAihuCssProvider(compile)

    expect(
      provider({
        source: '@template { <button>Save</button> }',
        id: '/src/Button.aihu',
        shadowMode: 'shadow',
        target: 'universal',
        lightScopeId: 'should-not-cross-the-boundary',
      }),
    ).toBe(':host { color: red; }')
    expect(compile).toHaveBeenCalledWith(
      '@template { <button>Save</button> }',
      '/src/Button.aihu',
      undefined,
    )
  })

  it('exports a ready-to-use provider with the public compiler type', () => {
    expect(typeof aihuCssProvider).toBe('function')
  })
})
