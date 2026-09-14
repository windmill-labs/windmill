import { describe, expect, it } from 'vitest'
import {
	artifactsFingerprint,
	headSig,
	packRequests,
	planSessionPush,
	type ChatSnapshot,
	type MirrorSyncState
} from './sessionMirrorPlan'
import type { Session } from './sessionState.svelte'

function session(over: Partial<Session> = {}): Session {
	return { id: 's1', name: 'session-1', createdAt: 1, workspace_id: 'ws', chatId: 'c1', ...over }
}

function chat(id: string, lastModified: number, imageIds: string[] = []): ChatSnapshot {
	return { id, lastModified, record: { id, lastModified }, imageIds }
}

const noArtifacts = { items: [], versions: [] }

function synced(over: Partial<MirrorSyncState> = {}): MirrorSyncState {
	return {
		id: 's1',
		ws: 'ws',
		head: headSig(session()),
		chats: { c1: 10 },
		images: { i1: 'c1' },
		artifacts: artifactsFingerprint(noArtifacts),
		...over
	}
}

describe('planSessionPush', () => {
	it('pushes everything for a session never backed up, and nothing for an unsent draft', () => {
		const plan = planSessionPush({
			session: session(),
			chats: [chat('c1', 10, ['i1'])],
			artifacts: noArtifacts
		})
		expect(plan?.entry?.head?.id).toBe('s1')
		expect(plan?.entry?.chats?.map((c) => c.id)).toEqual(['c1'])
		expect(plan?.images).toEqual([{ chat_id: 'c1', id: 'i1' }])
		// Nothing to store yet, so no artifacts object either.
		expect(plan?.entry?.artifacts).toBeUndefined()
		expect(plan?.next).toEqual(synced())

		expect(
			planSessionPush({
				session: session({ workspace_id: undefined, pending_workspace_id: 'ws' }),
				chats: [],
				artifacts: noArtifacts
			})
		).toBeUndefined()
	})

	it('sends nothing when only the fields reading a session bumps changed', () => {
		const plan = planSessionPush({
			session: session({ lastSeenCount: 7, lastActivityAt: 99, name: 'session-9' }),
			chats: [chat('c1', 10, ['i1'])],
			artifacts: noArtifacts,
			sync: synced()
		})
		expect(plan?.entry).toBeUndefined()
		expect(plan?.images).toEqual([])
	})

	it('carries only the chat whose lastModified moved, plus its new images', () => {
		const plan = planSessionPush({
			session: session(),
			chats: [chat('c1', 10, ['i1']), chat('c2', 20, ['i2'])],
			artifacts: noArtifacts,
			sync: synced()
		})
		expect(plan?.entry?.head).toBeUndefined()
		expect(plan?.entry?.chats?.map((c) => c.id)).toEqual(['c2'])
		expect(plan?.images).toEqual([{ chat_id: 'c2', id: 'i2' }])
		expect(plan?.next.chats).toEqual({ c1: 10, c2: 20 })
		expect(plan?.next.images).toEqual({ i1: 'c1', i2: 'c2' })
	})

	it('deletes a chat that is gone and an image its chat evicted', () => {
		const plan = planSessionPush({
			session: session(),
			chats: [chat('c1', 11, [])],
			artifacts: noArtifacts,
			sync: synced({ chats: { c1: 10, c2: 20 }, images: { i1: 'c1', i2: 'c2' } })
		})
		expect(plan?.entry?.delete_chats).toEqual(['c2'])
		// i2 goes with c2 server-side; only c1's evicted image is deleted on its own.
		expect(plan?.entry?.delete_images).toEqual([{ chat_id: 'c1', id: 'i1' }])
	})

	it('moves a session as a full push to the new workspace and a removal from the old', () => {
		const plan = planSessionPush({
			session: session({ workspace_id: 'ws2' }),
			chats: [chat('c1', 10, ['i1'])],
			artifacts: noArtifacts,
			sync: synced()
		})
		expect(plan?.workspaceId).toBe('ws2')
		expect(plan?.removeFrom).toBe('ws')
		expect(plan?.entry?.head?.workspace_id).toBe('ws2')
		expect(plan?.entry?.chats?.map((c) => c.id)).toEqual(['c1'])
		expect(plan?.images).toEqual([{ chat_id: 'c1', id: 'i1' }])
		expect(plan?.next.ws).toBe('ws2')
	})

	it('pushes artifacts when their fingerprint changes, including emptying them', () => {
		const items = [
			{
				id: 'a1',
				sessionId: 's1',
				kind: 'md' as const,
				name: 'notes',
				content: 'x',
				createdAt: 1,
				updatedAt: 2,
				version: 1
			}
		]
		const withArtifact = planSessionPush({
			session: session(),
			chats: [chat('c1', 10, ['i1'])],
			artifacts: { items, versions: [] },
			sync: synced()
		})
		expect(withArtifact?.entry?.artifacts).toEqual({ items, versions: [] })

		const emptied = planSessionPush({
			session: session(),
			chats: [chat('c1', 10, ['i1'])],
			artifacts: noArtifacts,
			sync: synced({ artifacts: artifactsFingerprint({ items, versions: [] }) })
		})
		expect(emptied?.entry?.artifacts).toEqual(noArtifacts)
	})
})

describe('packRequests', () => {
	const limits = { targetBytes: 200, maxSessions: 3, maxRemoved: 2 }
	const small = (id: string) => ({ id, head: { id } })

	it('stays within the per-request counts and spreads removals over requests', () => {
		const requests = packRequests(
			'me',
			[small('img')],
			[small('a'), small('b'), small('c')],
			['r1', 'r2', 'r3'],
			limits
		)
		expect(requests.map((r) => r.sessions.map((s) => s.id))).toEqual([['img', 'a', 'b'], ['c']])
		expect(requests.map((r) => r.removed)).toEqual([['r1', 'r2'], ['r3']])
		expect(packRequests('me', [], [], ['r1', 'r2', 'r3'], limits).map((r) => r.removed)).toEqual([
			['r1', 'r2'],
			['r3']
		])
	})

	it('splits an oversized entry into chat-only parts, the head riding on the last', () => {
		const big = (id: string) => ({ id, record: { id, text: 'x'.repeat(150) } })
		const entry = {
			id: 's',
			head: { id: 's' },
			chats: [big('c1'), big('c2'), big('c3')],
			delete_chats: ['old']
		}
		const requests = packRequests('me', [], [entry], [], limits)
		const parts = requests.flatMap((r) => r.sessions)
		expect(parts.map((p) => p.chats?.map((c) => c.id))).toEqual([['c1'], ['c2'], ['c3']])
		expect(
			parts.slice(0, -1).every((p) => p.head === undefined && p.delete_chats === undefined)
		).toBe(true)
		expect(parts.at(-1)?.head).toEqual({ id: 's' })
		expect(parts.at(-1)?.delete_chats).toEqual(['old'])
		// Each part outgrows the target on its own, so each travels alone.
		expect(requests.length).toBe(3)
	})
})
