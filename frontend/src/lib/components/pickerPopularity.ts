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

type HubResourceTypeInfo = { name: string; app: string; picks: number }

const hubInfoCached = createCache(
	async ({ workspace }: { workspace: string }): Promise<HubResourceTypeInfo[]> => {
		try {
			return await ResourceService.listHubResourceTypeInfo({ workspace })
		} catch {
			return []
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
export async function hubResourceTypePicks(workspace: string): Promise<PopularityCounts> {
	if (get(disableHubStore)) return {}
	const info = await hubInfoCached({ workspace })
	return Object.fromEntries(info.map((rt) => [rt.name, rt.picks]))
}

/**
 * How many resources of each type this workspace holds — the only evidence about this
 * particular team. Keyed by resource type, which is what the add-resource drawer lists.
 */
export function localResourceTypeCounts(workspace: string): Promise<PopularityCounts> {
	return localCountsCached({ workspace })
}

/**
 * The same counts totalled per integration, which is what the flow step picker lists.
 *
 * A type usually shares its integration's name, but often enough it does not:
 * `discord_webhook` and `discord_bot_configuration` are both Discord, `ms_teams_webhook` and
 * `azure_bot` are both MS Teams. Only the hub knows that, so a workspace whose Discord
 * credential is a `discord_webhook` would otherwise read as one that has never touched
 * Discord — and since local usage is the leading tier, that decides which half of the list
 * the integration lands in, not merely its position within one.
 *
 * A type the hub has no mapping for counts under its own name, which is the right guess and
 * also what an unreachable hub leaves every type with.
 */
export async function localCountsByIntegration(workspace: string): Promise<PopularityCounts> {
	const [counts, info] = await Promise.all([
		localCountsCached({ workspace }),
		get(disableHubStore) ? Promise.resolve([]) : hubInfoCached({ workspace })
	])
	return totalLocalCountsByApp(counts, info)
}

/** The mapping half of {@link localCountsByIntegration}, separated so it can be tested alone. */
export function totalLocalCountsByApp(
	counts: PopularityCounts,
	hub: { name: string; app: string }[]
): PopularityCounts {
	const appOf = new Map(hub.map((rt) => [rt.name, rt.app]))
	const byApp: PopularityCounts = {}
	for (const [name, count] of Object.entries(counts)) {
		const app = appOf.get(name) ?? name
		byApp[app] = (byApp[app] ?? 0) + count
	}
	return byApp
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
 * Four tiers. **Whether this workspace already holds a resource of the type leads**, then the
 * hub's pick count, then how many local resources there are, then the name.
 *
 * Used-here leads rather than merely breaking hub ties because the two counts are on
 * incomparable scales: a hub pick count is global and grows without bound, a local count is
 * usually single digits. Ranked the other way round, local usage only ever sorts the slice
 * where hub counts are equal — which, since they are distinct integers, is just the tail
 * that nobody has picked. That reads fine on a hub with few picks and silently stops
 * mattering as one fills up, so the ordering would drift away from the workspace's own
 * stack with no change to this code.
 *
 * Within each half the hub decides, so "yours" and "everyone's" are both honoured rather
 * than blended with a weighting constant that would need tuning. Alphabetical is the floor,
 * and it is where an entry neither signal knows about lands.
 */
export function byPopularity(
	hub: PopularityCounts,
	local: PopularityCounts
): (a: string, b: string) => number {
	const usedHere = (name: string) => ((local[name] ?? 0) > 0 ? 1 : 0)
	return (a, b) =>
		usedHere(b) - usedHere(a) ||
		(hub[b] ?? 0) - (hub[a] ?? 0) ||
		(local[b] ?? 0) - (local[a] ?? 0) ||
		a.localeCompare(b)
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
