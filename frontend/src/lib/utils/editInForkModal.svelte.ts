import { createState } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import type { ItemType } from './editInFork'

/**
 * An "Edit in <dev workspace>" click that landed on an item the dev workspace
 * doesn't have yet. Held globally so the confirmation renders once in the logged
 * layout instead of per item row.
 */
export type PullIntoDevModalState = {
	itemType: ItemType
	itemPath: string
	devWorkspaceId: string
	devWorkspaceName: string
	prodWorkspaceId: string
}

export let pullIntoDevModal: StateStore<PullIntoDevModalState | undefined> = createState({
	val: undefined
})
