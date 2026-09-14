import { get } from 'svelte/store'
import { IntegrationService, ResourceService } from '$lib/gen'
import { disableHubStore } from '$lib/stores'
import { createCache } from '$lib/utils'
import { setHubIntegrationDisplayNames, setResourceTypeDisplayNames } from './resourceTypeDisplay'

/**
 * Loads the names `resourceTypeDisplayName` and `integrationDisplayName` read, for a surface that
 * shows a label without already fetching the rows it comes from. Apart from `resourceTypeDisplay`,
 * which makes no API calls so it can be unit-tested alone. Cached briefly: drawers and pickers
 * reopen often, and a name rarely changes.
 */
const CACHE_MS = 60_000

const resourceTypeRowCached = createCache(
	({ workspace, name }: { workspace: string; name: string }) =>
		ResourceService.getResourceType({ workspace, path: name }).then(
			(rt) => setResourceTypeDisplayNames([rt]),
			() => {}
		),
	{ invalidateMs: CACHE_MS, maxSize: 50 }
)

/**
 * Fill `resourceTypeDisplayName` for one type, for a surface titled with a type it holds no row
 * for. The name is stored with the type, so this reads the row rather than the hub.
 */
export function loadResourceTypeDisplayName(workspace: string, name: string): Promise<void> {
	return resourceTypeRowCached({ workspace, name })
}

const hubIntegrationNamesCached = createCache(
	(_: Record<string, never>) =>
		IntegrationService.listHubIntegrations().then(setHubIntegrationDisplayNames, () => {}),
	{ invalidateMs: CACHE_MS }
)

/**
 * Fill `integrationDisplayName` for a picker whose integrations come from its own items rather
 * than the hub's integration list, as the hub app and flow pickers do. Unfiltered: `kind`
 * narrows by script kind, so asking for an app or a flow would name nothing.
 */
export function loadHubIntegrationDisplayNames(): Promise<void> {
	if (get(disableHubStore)) return Promise.resolve()
	return hubIntegrationNamesCached({})
}
