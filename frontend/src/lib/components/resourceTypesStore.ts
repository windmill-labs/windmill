import { ResourceService } from "$lib/gen";
import { workspaceStore } from "$lib/stores";
import { get, writable } from "svelte/store";

export const resourceTypesStore = writable<string[] | undefined>(undefined)

export async function getResourceTypes() {
    const rts = get(resourceTypesStore)
    if (rts) {
        return rts
    } else {
        const nrts = await ResourceService.listResourceTypeNames({ workspace: get(workspaceStore)! })
        resourceTypesStore.set(nrts)
        return nrts
    }
}


