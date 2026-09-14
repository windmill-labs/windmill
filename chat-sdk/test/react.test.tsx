import { GlobalRegistrator } from '@happy-dom/global-registrator'
// Test files share one process: the DOM globals must not outlive this file.
GlobalRegistrator.register()

import { afterAll, describe, expect, test } from 'bun:test'
import React, { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { useWindmillChat, type UseWindmillChat } from '../src/react'
import type { ChatOptions } from '../src/types'
import { fetchMock, memoryStorage } from './support'

afterAll(() => GlobalRegistrator.unregister())

const base: ChatOptions = {
  flowPath: 'f/chat/agent',
  baseUrl: 'http://wm.test',
  workspace: 'ws',
  fetch: fetchMock().fetch,
  storage: memoryStorage()
}

/** Renders the hook and hands back what it returned, rerendering with new options on demand. */
function mountHook() {
  let latest: UseWindmillChat | undefined
  const Probe = (props: ChatOptions) => {
    latest = useWindmillChat(props)
    return null
  }
  const root: Root = createRoot(document.createElement('div'))
  const render = (props: ChatOptions) => {
    act(() => root.render(<Probe {...props} />))
    return latest!
  }
  return { render, unmount: () => act(() => root.unmount()) }
}

describe('useWindmillChat', () => {
  test('a credential change is a new chat; a new closure for the same credential is not', () => {
    const { render, unmount } = mountHook()
    const a = render({ ...base, token: 'user-a' }).chat
    expect(render({ ...base, token: 'user-a' }).chat).toBe(a)
    const b = render({ ...base, token: 'user-b' }).chat
    expect(b).not.toBe(a)

    const fn1 = render({ ...base, token: () => 'fn-1' }).chat
    expect(fn1).not.toBe(b)
    expect(render({ ...base, token: () => 'fn-2' }).chat).toBe(fn1)

    const session = render({ ...base }).chat
    expect(session).not.toBe(fn1)
    const fn3 = render({ ...base, token: () => 'fn-3' }).chat
    expect(fn3).not.toBe(session)
    expect(render({ ...base, token: () => 'fn-3', storageKey: 'someone-else' }).chat).not.toBe(fn3)
    unmount()
  })

  test('the latest inputs go with the next message', async () => {
    const { fetch, calls } = fetchMock((c) => (c.method === 'POST' ? new Response('job-1') : undefined))
    const { render, unmount } = mountHook()
    render({ ...base, fetch, token: 'tok', inputs: { docId: 'first' } })
    const hook = render({ ...base, fetch, token: 'tok', inputs: { docId: 'second' } })
    // The run's stream never answers here; only the request matters.
    void hook.sendMessage('hi', { inputs: { extra: true } }).catch(() => {})
    await new Promise((r) => setTimeout(r, 20))
    expect(calls.find((c) => c.method === 'POST')?.body).toEqual({ docId: 'second', extra: true, user_message: 'hi' })
    hook.chat.destroy()
    // A key the latest render no longer passes is gone from the next message.
    const cleared = render({ ...base, fetch, token: 'tok', inputs: {} })
    void cleared.sendMessage('again').catch(() => {})
    await new Promise((r) => setTimeout(r, 20))
    expect(calls.filter((c) => c.method === 'POST')[1]?.body).toEqual({ user_message: 'again' })
    unmount()
  })

  test('a token function is read through a ref, so the latest closure serves the next request', async () => {
    const { fetch, calls } = fetchMock((c) => (c.url.pathname.includes('/flow_conversations/list') ? new Response('[]') : undefined))
    const { render, unmount } = mountHook()
    const first = render({ ...base, fetch, history: 'server', token: () => 'first' })
    await act(() => first.loadConversations())
    const second = render({ ...base, fetch, history: 'server', token: () => 'second' })
    expect(second.chat).toBe(first.chat)
    await act(() => second.loadConversations())
    expect(calls.map((c) => c.headers.authorization)).toEqual(['Bearer first', 'Bearer second'])
    unmount()
  })
})
