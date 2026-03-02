import { ResourceService } from "$lib/gen";
import { workspaceStore } from "$lib/stores";
import { get, writable } from "svelte/store";

export const resourceTypesStore = writable<string[] | undefined>(undefined)

let loading = false

export async function getResourceTypes() {
    const rts = get(resourceTypesStore)
    if (rts) {
        return rts
    } else {
        if (loading) {
            let counter = 0
            while (loading) {
                await new Promise(resolve => setTimeout(resolve, 50))
                counter++
                if (counter > 10) {
                    console.error('Resource types loading timeout')
                    break
                }
            }
            if (!loading) {
                return get(resourceTypesStore)
            }

        }
        loading = true

        let workspace = get(workspaceStore)
        if (workspace) {
            try {
                const nrts = await ResourceService.listResourceTypeNames({ workspace: workspace })
                resourceTypesStore.set(nrts)
                loading = false
                return nrts
            } catch (e) {
                loading = false
                return ["error_fetching_names"]
            }
        } else {
            return ['workspace_is_undefined']
        }
    }
}


