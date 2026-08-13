import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({ OpenAPI: { BASE: '/api' } }))
vi.mock('$lib/stores', () => ({ workspaceStore: { subscribe: () => () => {} } }))

import { createFeatureUsageBuffer, type FeatureUsageEventPayload } from './featureUsage'

describe('createFeatureUsageBuffer', () => {
	it('sums repeated events per (feature, kind, key, entity) and flushes one batch', async () => {
		const send = vi.fn().mockResolvedValue(undefined)
		const buffer = createFeatureUsageBuffer(send, () => 'ws1')

		buffer.log('ai_session', 'message', { key: 'global', entityId: 's1' })
		buffer.log('ai_session', 'message', { key: 'global', entityId: 's1' })
		buffer.log('ai_session', 'tokens', { entityId: 's1', value: 120 })
		buffer.log('ai_session', 'message', { key: 'global', entityId: 's2' })
		await buffer.flush()

		expect(send).toHaveBeenCalledTimes(1)
		const [workspace, events] = send.mock.calls[0]
		expect(workspace).toBe('ws1')
		expect(events).toEqual(
			expect.arrayContaining([
				{ feature: 'ai_session', kind: 'message', key: 'global', entity_id: 's1', value: 2 },
				{ feature: 'ai_session', kind: 'tokens', key: '', entity_id: 's1', value: 120 },
				{ feature: 'ai_session', kind: 'message', key: 'global', entity_id: 's2', value: 1 }
			])
		)
		expect(events).toHaveLength(3)

		// Flushed events must not be re-sent.
		await buffer.flush()
		expect(send).toHaveBeenCalledTimes(1)
	})

	it('splits batches per workspace and drops events without any workspace', async () => {
		const send = vi.fn().mockResolvedValue(undefined)
		const buffer = createFeatureUsageBuffer(send, () => undefined)

		buffer.log('ai_session', 'created', { key: 'fork' }) // no workspace -> dropped
		buffer.log('ai_session', 'created', { key: 'fork', workspace: 'ws1' })
		buffer.log('ai_session', 'created', { key: 'root', workspace: 'ws2' })
		await buffer.flush()

		expect(send).toHaveBeenCalledTimes(2)
		const workspaces = send.mock.calls.map((c) => c[0]).sort()
		expect(workspaces).toEqual(['ws1', 'ws2'])
	})

	it('starts every chunk request before any send resolves (pagehide flush)', async () => {
		const send = vi.fn().mockReturnValue(new Promise<void>(() => {}))
		const buffer = createFeatureUsageBuffer(send, () => 'ws1')

		for (let i = 0; i < 60; i++) {
			buffer.log('ai_session', 'tool', { key: `tool_${i}` })
		}
		buffer.log('ai_session', 'message', { workspace: 'ws2' })
		void buffer.flush()
		await Promise.resolve()

		// keepalive only protects requests that were issued; a sequential flush
		// would have started just the first chunk here.
		expect(send).toHaveBeenCalledTimes(3)
	})

	it('chunks flushes above the per-request cap and survives send failures', async () => {
		const send = vi.fn().mockRejectedValueOnce(new Error('network')).mockResolvedValue(undefined)
		const buffer = createFeatureUsageBuffer(send, () => 'ws1')

		for (let i = 0; i < 60; i++) {
			buffer.log('ai_session', 'tool', { key: `tool_${i}` })
		}
		await expect(buffer.flush()).resolves.toBeUndefined()

		expect(send).toHaveBeenCalledTimes(2)
		const sent = send.mock.calls.flatMap((c) => c[1] as FeatureUsageEventPayload[])
		expect(send.mock.calls[0][1]).toHaveLength(50)
		expect(sent).toHaveLength(60)
	})
})
