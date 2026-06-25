import { base } from '$lib/base'
import { get } from 'svelte/store'
import { userWorkspaces, workspaceStore } from '$lib/stores'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'

type ItemType = 'script' | 'flow' | 'app' | 'raw_app'

function editPathFor(itemType: ItemType, itemPath: string): string {
	switch (itemType) {
		case 'script':
			return `${base}/scripts/edit/${itemPath}`
		case 'flow':
			return `${base}/flows/edit/${itemPath}`
		case 'app':
			return `${base}/apps/edit/${itemPath}`
		case 'raw_app':
			return `${base}/apps_raw/edit/${itemPath}`
	}
}

export function buildForkEditUrl(itemType: ItemType, itemPath: string): string {
	const editPath = editPathFor(itemType, itemPath)
	// When the current ("prod") workspace has a canonical dev workspace, edits are funneled there:
	// deep-link straight into the dev workspace's editor instead of the create-fork wizard.
	const dev = findCanonicalDevWorkspace(get(workspaceStore), get(userWorkspaces))
	if (dev) {
		return `${editPath}?workspace=${encodeURIComponent(dev.id)}`
	}
	return `${base}/user/fork_workspace?rd=${encodeURIComponent(editPath)}`
}
