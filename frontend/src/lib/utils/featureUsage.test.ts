import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({ OpenAPI: { BASE: '/api' } }))

// Stores `get()` can read, so a test can say which hub the instance points at and whether
// the instance has answered at all.
const hubBaseUrl = vi.hoisted(() => {
	const readable = <T>(initial: T) => {
		let value = initial
		return {
			set: (v: T) => (value = v),
			store: {
				subscribe: (run: (v: T) => void) => {
					run(value)
					return () => {}
				}
			}
		}
	}
	return { url: readable('https://hub.windmill.dev'), known: readable(true) }
})

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => {} },
	hubBaseUrlStore: hubBaseUrl.url.store,
	hubBaseUrlKnown: hubBaseUrl.known.store
}))

import {
	createFeatureUsageBuffer,
	hubProjectUsageKey,
	hubScriptUsageKey,
	type FeatureUsageEventPayload
} from './featureUsage'

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

describe('hubScriptUsageKey', () => {
	it('names a public hub script by app and summary', () => {
		expect(
			hubScriptUsageKey({ version_id: 9084, app: 'slack', summary: 'Send message to channel' })
		).toBe('slack/send_message_to_channel')
	})

	it('never reports a private hub script name', () => {
		// Above PRIVATE_HUB_MIN_VERSION the app and summary are the customer's own.
		expect(
			hubScriptUsageKey({
				version_id: 10_000_001,
				app: 'acme_internal',
				summary: 'Payroll export'
			})
		).toBe('private')
	})

	it('slugifies punctuation rather than filing the script under private', () => {
		expect(
			hubScriptUsageKey({ version_id: 12, app: 'acme', summary: "List a user's items, sorted" })
		).toBe('acme/list_a_user_s_items_sorted')
	})
})

describe('hubProjectUsageKey', () => {
	// The fixture is module-level and mutable, so each case states the world it needs rather
	// than inheriting whatever the case above it left behind.
	beforeEach(() => {
		hubBaseUrl.url.set('https://hub.windmill.dev')
		hubBaseUrl.known.set(true)
	})

	it('reports the slug for every spelling of the public hub', () => {
		for (const hub of [
			'https://hub.windmill.dev',
			'http://hub.windmill.dev/',
			'HTTPS://hub.windmill.dev',
			'https://HUB.WINDMILL.DEV',
			'https://hub.windmill.dev:443',
			'  https://hub.windmill.dev  '
		]) {
			hubBaseUrl.url.set(hub)
			expect(hubProjectUsageKey('stripe-invoices'), hub).toBe('stripe-invoices')
		}
	})

	it('answers private until the instance has said which hub it points at', () => {
		// The store is seeded with the public hub, so a settings read that failed must not
		// read as permission to report the name.
		hubBaseUrl.known.set(false)
		expect(hubProjectUsageKey('acme-payroll')).toBe('private')
		hubBaseUrl.known.set(true)
		expect(hubProjectUsageKey('acme-payroll')).toBe('acme-payroll')
	})

	it("keeps a private hub's project names off the wire", () => {
		// The slug is the customer's own content on an instance running its own hub, and the
		// disclosure only claims public project names.
		for (const hub of [
			'https://hub.internal.example',
			'https://hub.windmill.dev.evil.example',
			'https://windmill.dev',
			'hub.windmill.dev',
			'not a url'
		]) {
			hubBaseUrl.url.set(hub)
			expect(hubProjectUsageKey('acme-payroll'), hub).toBe('private')
		}
	})
})
