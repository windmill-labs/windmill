import { describe, expect, it, vi } from 'vitest'
import type { AiArtifactListEntry } from '$lib/gen'
import type { PersistedArtifact } from './artifactsDB'

vi.mock('$lib/components/sessions/sessionState.svelte', () => ({ sessionState: { sessions: [] } }))
vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { userStore: writable(undefined), userWorkspaces: writable([]) }
})
vi.mock('$lib/utils', () => ({ getLocalSetting: () => undefined, storeLocalSetting: () => {} }))
vi.mock('$lib/gen', () => ({ AiService: {} }))

import { mergeArtifactRows } from './artifactRows'

describe('mergeArtifactRows', () => {
	it("takes an artifact's trash state from this browser's copy", () => {
		// A plan written again in its session is live here while the server copy it was shared from
		// stays in the trash. Listed in Recently deleted, Delete forever would take the plan with it.
		const plan = local({ id: 'p', role: 'plan' })
		const trashed = server({ artifact_id: 'p', deleted_at: '2026-11-01T00:00:00Z' })
		expect(mergeArtifactRows([plan], [trashed], true)).toEqual([])
		const [row] = mergeArtifactRows([plan], [trashed])
		expect(row).toMatchObject({ local: plan, deletedAt: undefined, extraPerms: undefined })
	})
})

function local(over: Partial<PersistedArtifact> = {}): PersistedArtifact {
	return {
		id: 'a1',
		sessionId: 's1',
		kind: 'md',
		name: 'Doc',
		content: '# hi',
		createdAt: 0,
		updatedAt: 0,
		workspaceId: 'ws',
		...over
	}
}

function server(over: Partial<AiArtifactListEntry> = {}): AiArtifactListEntry {
	return {
		artifact_id: 'a1',
		path: 'u/admin/doc',
		extra_perms: { 'u/bob': false },
		owner: 'admin',
		is_owner: true,
		owner_is_member: true,
		name: 'Doc',
		kind: 'md',
		shown_version: 1,
		latest_version: 1,
		pinned: false,
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...over
	}
}
