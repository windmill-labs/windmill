import { beforeEach, describe, expect, it } from 'vitest'
import { globalDraftStore } from './draftStore.svelte'

const WORKSPACE_A = 'draft-store-test-a'
const WORKSPACE_B = 'draft-store-test-b'

function clearTestDrafts() {
	globalDraftStore.clearDrafts(WORKSPACE_A)
	globalDraftStore.clearDrafts(WORKSPACE_B)
}

describe('globalDraftStore', () => {
	beforeEach(clearTestDrafts)

	it('lists and reads drafts only from the requested workspace', () => {
		globalDraftStore.setDraft(WORKSPACE_A, {
			type: 'script',
			path: 'f/shared/path',
			language: 'bun',
			value: 'export async function main() {}',
			isDraft: true
		})

		expect(globalDraftStore.getDraft(WORKSPACE_A, 'script', 'f/shared/path')?.value).toBe(
			'export async function main() {}'
		)
		expect(globalDraftStore.getDraft(WORKSPACE_B, 'script', 'f/shared/path')).toBeUndefined()
		expect(globalDraftStore.listDrafts(WORKSPACE_A)).toHaveLength(1)
		expect(globalDraftStore.listDrafts(WORKSPACE_B)).toHaveLength(0)
	})

	it('deletes and clears drafts only from the requested workspace', () => {
		globalDraftStore.setDraft(WORKSPACE_A, {
			type: 'flow',
			path: 'f/shared/path',
			value: { value: { modules: [] }, schema: null, groups: null },
			isDraft: true
		})
		globalDraftStore.setDraft(WORKSPACE_B, {
			type: 'flow',
			path: 'f/shared/path',
			value: { value: { modules: [] }, schema: { workspace: WORKSPACE_B }, groups: null },
			isDraft: true
		})

		globalDraftStore.deleteDraft(WORKSPACE_A, 'flow', 'f/shared/path')

		expect(globalDraftStore.getDraft(WORKSPACE_A, 'flow', 'f/shared/path')).toBeUndefined()
		expect(globalDraftStore.getDraft(WORKSPACE_B, 'flow', 'f/shared/path')).toBeDefined()

		globalDraftStore.clearDrafts(WORKSPACE_B)

		expect(globalDraftStore.listDrafts(WORKSPACE_A)).toHaveLength(0)
		expect(globalDraftStore.listDrafts(WORKSPACE_B)).toHaveLength(0)
	})
})
