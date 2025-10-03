import { base } from "$lib/base"
import { workspaceStore } from "$lib/stores"
import { get } from "svelte/store"

export function computeSecretUrl(secretUrl: string) {
    return `${window.location.origin}${base}/public/${get(workspaceStore)}/${secretUrl}`
}
