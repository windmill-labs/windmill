import { get } from 'svelte/store'
import { ResourceService } from '$lib/gen'
import { disableHubStore } from '$lib/stores'
import { createCache } from '$lib/utils'
import { isCustomResourceTypeName } from './resourceTypeDisplay'

/**
 * How often something has been picked or used, keyed by integration or resource type name.
 * A name the caller lists but this map does not mention counts as zero, which is what an
 * unpicked entry and an absent signal both mean.
 */
export type PopularityCounts = Record<string, number>

/**
 * The signals are read on every picker open, so they are cached briefly; both resolve to an
 * empty map rather than rejecting, since an ordering hint is never worth a broken list.
 */
const CACHE_MS = 60_000

const hubPicksCached = createCache(
	async ({ workspace }: { workspace: string }): Promise<PopularityCounts> => {
		try {
			const picked = await ResourceService.listHubPickedResourceTypes({ workspace })
			return Object.fromEntries(picked.map((rt) => [rt.name, rt.picks]))
		} catch {
			return {}
		}
	},
	{ invalidateMs: CACHE_MS }
)

const localCountsCached = createCache(
	async ({ workspace }: { workspace: string }): Promise<PopularityCounts> => {
		try {
			const counts = await ResourceService.listResourceCountsByType({ workspace })
			return Object.fromEntries(counts.map((c) => [c.resource_type, c.count]))
		} catch {
			return {}
		}
	},
	{ invalidateMs: CACHE_MS }
)

/**
 * What the hub sees people pick, per resource type. Empty on a hub that counts nothing,
 * and on an instance that has switched the hub off — a closed environment must not spend a
 * request on hub.windmill.dev just to order a list.
 */
export function hubResourceTypePicks(workspace: string): Promise<PopularityCounts> {
	if (get(disableHubStore)) return Promise.resolve({})
	return hubPicksCached({ workspace })
}

/**
 * How many resources of each type this workspace holds — the only evidence about this
 * particular team, and the same map the flow step picker reads for an integration, since an
 * integration and the resource type authenticating it share a name.
 */
export function localResourceTypeCounts(workspace: string): Promise<PopularityCounts> {
	return localCountsCached({ workspace })
}

/**
 * Tell the hub a resource type was taken into a workspace, which is what its ranking counts.
 * Fire-and-forget: a hub that does not count picks must not be felt by the user who just
 * saved a resource. Workspace-made types exist on no hub, so they are not reported.
 */
export function recordHubResourceTypePick(workspace: string, resourceType: string): void {
	if (get(disableHubStore)) return
	if (!resourceType || isCustomResourceTypeName(resourceType)) return
	ResourceService.pickHubResourceType({ workspace, name: resourceType }).catch(() => {})
}

/**
 * Orders the lists that offer hub content: integrations in the flow step picker, resource
 * types in the add-resource drawer.
 *
 * What the hub sees people pick ranks first, being drawn from every Windmill instance; what
 * this workspace already uses breaks its ties. Alphabetical is the floor, and it is where a
 * hub that ranks nothing leaves every entry it has no count for.
 */
export function byPopularity(
	hub: PopularityCounts,
	local: PopularityCounts
): (a: string, b: string) => number {
	return (a, b) =>
		(hub[b] ?? 0) - (hub[a] ?? 0) || (local[b] ?? 0) - (local[a] ?? 0) || a.localeCompare(b)
}

/**
 * The ordering to hold before either signal has landed: the alphabetical floor, which is
 * what `byPopularity` degrades to anyway.
 *
 * A list has to be sorted by *something* from its first paint — one source of these names
 * is a `HashMap` on the server, so leaving them unsorted means hash order, which differs
 * between processes.
 */
export const alphabetical = byPopularity({}, {})
