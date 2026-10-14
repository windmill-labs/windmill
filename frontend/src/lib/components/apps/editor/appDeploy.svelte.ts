import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

export function computeSecretUrl(secretUrl: string) {
	return `${window.location.origin}/public/${get(workspaceStore)}/${secretUrl}`
}
