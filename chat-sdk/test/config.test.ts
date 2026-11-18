import { afterEach, describe, expect, test } from 'bun:test'
import { resolveConfig } from '../src/config'

const g = globalThis as { process?: unknown; ctx?: unknown; location?: unknown }
const originalProcess = g.process

afterEach(() => {
  g.process = originalProcess
  delete g.ctx
  delete g.location
})

describe('resolveConfig', () => {
  test('explicit token defaults history to local; a session defaults to server', () => {
    const base = { flowPath: 'f/a/b', baseUrl: 'http://wm.test/', workspace: 'ws' }
    expect(resolveConfig({ ...base, token: 't' }).history).toBe('local')
    expect(resolveConfig(base).history).toBe('server')
    expect(resolveConfig({ ...base, token: 't', history: 'server' }).historyExplicit).toBe(true)
  })

  test('reads the sandboxed raw app env the wrapper injects', () => {
    g.process = {
      env: { WM_RAW_APP: 'true', WM_TOKEN: 'sdk-token', BASE_URL: 'http://wm.test', WM_WORKSPACE: 'ws' }
    }
    const config = resolveConfig({ flowPath: 'f/a/b' })
    expect(config).toMatchObject({ baseUrl: 'http://wm.test', workspace: 'ws', token: 'sdk-token', history: 'server' })
  })

  test('keeps the raw app token off another instance', () => {
    g.process = {
      env: { WM_RAW_APP: 'true', WM_TOKEN: 'sdk-token', BASE_URL: 'http://wm.test', WM_WORKSPACE: 'ws' }
    }
    expect(resolveConfig({ flowPath: 'f/a/b', baseUrl: 'http://other.test', workspace: 'ws' }).token).toBeUndefined()
    expect(resolveConfig({ flowPath: 'f/a/b', baseUrl: 'http://wm.test' }).token).toBe('sdk-token')
  })

  test('reads the unsandboxed raw app context and uses the page origin', () => {
    g.ctx = { ctx: { username: 'admin' }, workspace: 'ws' }
    g.location = { origin: 'http://wm.test' }
    const config = resolveConfig({ flowPath: 'f/a/b' })
    expect(config).toMatchObject({ baseUrl: 'http://wm.test', workspace: 'ws', token: undefined })
  })

  test('refuses an opaque origin without an SDK token', () => {
    g.ctx = { workspace: 'ws' }
    g.location = { origin: 'null' }
    expect(() => resolveConfig({ flowPath: 'f/a/b' })).toThrow('frontend SDK scopes')
  })
})
