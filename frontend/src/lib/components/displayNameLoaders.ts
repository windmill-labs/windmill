import { get } from 'svelte/store'
import { IntegrationService, ResourceService } from '$lib/gen'
import { disableHubStore } from '$lib/stores'
import { createCache } from '$lib/utils'
import { setHubIntegrationDisplayNames, setResourceTypeDisplayNames } from './resourceTypeDisplay'

/**
 * Loads what `resourceTypeDisplayName` and `integrationDisplayName` read: a type's stored name for a
 * surface that holds no row for it, and the hub's integration list, which every picker that needs
 * it shares. Apart from `resourceTypeDisplay`, which makes no API calls so it can be unit-tested
 * alone. Cached briefly: drawers and pickers reopen often, and a name rarely changes.
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

const hubIntegrationsCached = createCache(
	({ kind }: { kind?: string }) =>
		IntegrationService.listHubIntegrations({ kind }).then((integrations) => {
			setHubIntegrationDisplayNames(integrations)
			return integrations
		}),
	{ invalidateMs: CACHE_MS }
)

/**
 * The hub's integration list, read once a minute per `kind` however many pickers ask, recording
 * each integration's name on the way. A failed read is kept for that minute too, and rejects, so
 * a picker can say the hub is unavailable.
 */
export function listHubIntegrationsShared(kind?: string) {
	return hubIntegrationsCached({ kind })
}

/**
 * Fill `integrationDisplayName` for a picker whose integrations come from its own items rather
 * than the hub's integration list, as the hub app and flow pickers do. Unfiltered: `kind`
 * narrows by script kind, so asking for an app or a flow would name nothing.
 */
export function loadHubIntegrationDisplayNames(): Promise<void> {
	if (get(disableHubStore)) return Promise.resolve()
	return listHubIntegrationsShared().then(
		() => {},
		() => {}
	)
}
