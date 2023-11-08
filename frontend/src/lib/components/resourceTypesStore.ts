import { ResourceService } from "$lib/gen";
import { workspaceStore } from "$lib/stores";
import { get, writable } from "svelte/store";

export const resourceTypesStore = writable<string[] | undefined>(undefined)

export async function getResourceTypes() {
    const rts = get(resourceTypesStore)
    if (rts) {
        return rts
    } else {
        let workspace = get(workspaceStore)
        if (workspace){
            try {
                const nrts = await ResourceService.listResourceTypeNames({ workspace: workspace })
                resourceTypesStore.set(nrts)
                return nrts
            } catch (e) {
                return ["error_fetching_names"]        
            }
        } else {
            return ['workspace_is_undefined']
        }
    }
}


