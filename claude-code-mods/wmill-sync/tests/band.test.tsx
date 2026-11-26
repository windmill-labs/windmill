import { expect, test } from 'claude-code/testing'

import type { SyncStatus } from '../types'

const STATUS: SyncStatus = {
  phase: 'ready',
  root: '/project',
  target: { remote: 'http://localhost:8000/', workspaceId: 'demo', source: 'profile "demo"' },
  checkedAt: 1,
  error: null,
  gitSync: null,
  items: [
    {
      key: 'flow:f/demo/pipeline',
      kind: 'flow',
      path: 'f/demo/pipeline',
      direction: 'conflict',
      why: 'edited locally, and changed on the workspace',
      files: ['f/demo/pipeline__flow/a.ts'],
    },
    {
      key: 'script:f/demo/greet',
      kind: 'script',
      path: 'f/demo/greet',
      direction: 'push',
      why: 'edited locally',
      files: ['f/demo/greet.ts'],
    },
  ],
}

const BAND = {
  component: 'AbovePrompt',
  props: {
    hasSurvey: false,
    isWorking: false,
    maxRows: 12,
    bodyColumns: 90,
    scroll: { offset: 0, bodyRows: 12 },
    view: {},
  },
} as const

test('the band lists out-of-sync items with their actions on every surface with buttons', async ($, on) => {
  // The kit hands a state.get answer's `value` back as the read itself, so
  // the read the plugin sees is nested one level down.
  const held = (value: unknown) => ({ value: { value, version: 1 }, version: 1 }) as never
  on('state.get', (_, e, next) => {
    if (e.plugin !== 'wmill-sync') return next(e)
    if (e.key === 'status') return held(STATUS)
    if (e.key === 'isExpanded') return held(true)
    return next(e)
  })
  for (const surface of ['terminal', 'desktop'] as const) {
    const ui = await $.ui.mount({ plugin: 'wmill-sync', surface, ...BAND })
    expect((await ui.find({ key: 'push-all' }))?.text).toBe('Push 1')
    expect(await ui.find({ key: 'resolve-flow:f/demo/pipeline' })).toBeDefined()
    expect(await ui.find({ key: 'push-flow:f/demo/pipeline' })).toBeUndefined()
    expect(await ui.find({ key: 'push-script:f/demo/greet' })).toBeDefined()
    expect(await ui.find({ type: 'Text', text: /1 to push · 1 conflict/ })).toBeDefined()
    await ui.unmount()
  }
})
