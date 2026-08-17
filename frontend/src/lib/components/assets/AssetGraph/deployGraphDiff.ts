import type { AssetGraphResponse } from './types'
import { assetKey, isWriteEdge } from './lib'

// Post-deploy graph verification: the canvas previews a draft's edges from
// the *frontend* parsers (resolveGraph overlay); the deployed rows are
// derived by the *backend* at create_script. The two are kept behaviorally
// identical, but if they ever drift the failure mode is brutal — the chain
// the user tested isn't the chain they shipped, discovered only when a
// production cascade silently doesn't fire. So right after a deploy +
// graph refetch, we diff what the preview promised against what actually
// persisted and surface any gap immediately.
//
// Only the cascade-relevant facts are compared: write edges (what makes
// this script a producer) and `// on` asset subscriptions (what makes it a
// subscriber). Read edges are informational lineage and reads missing from
// either side don't change execution.

export type CascadeFacts = {
	/** `kind:path` of assets this script writes (access w/rw). */
	writes: Set<string>
	/** `kind:path` of assets this script subscribes to (`// on`). */
	subs: Set<string>
}

export function extractCascadeFacts(g: AssetGraphResponse, scriptPath: string): CascadeFacts {
	const writes = new Set<string>()
	for (const e of g.edges ?? []) {
		if (e.runnable_path !== scriptPath || !isWriteEdge(e)) continue
		writes.add(assetKey(e))
	}
	const subs = new Set<string>()
	for (const t of g.triggers ?? []) {
		if (t.trigger_kind !== 'asset') continue
		if (t.runnable_kind !== 'script' || t.runnable_path !== scriptPath) continue
		subs.add(assetKey(t))
	}
	return { writes, subs }
}

export type DeployGraphDrift = {
	scriptPath: string
	/** Promised by the preview, absent after deploy — the dangerous direction. */
	missingWrites: string[]
	missingSubs: string[]
}

/**
 * Compare per-script predictions (captured from the resolved draft graph
 * BEFORE deploy) against the refetched persisted graph. Returns one entry
 * per script that lost a promised fact; empty array = deploy matched the
 * preview. Extra server-side facts (backend parser saw more than the
 * preview) are not drift — the union at deploy makes that direction normal.
 */
export function diffDeployedGraph(
	predicted: Map<string, CascadeFacts>,
	deployed: AssetGraphResponse
): DeployGraphDrift[] {
	const out: DeployGraphDrift[] = []
	for (const [scriptPath, want] of predicted) {
		const got = extractCascadeFacts(deployed, scriptPath)
		const missingWrites = [...want.writes].filter((w) => !got.writes.has(w))
		const missingSubs = [...want.subs].filter((s) => !got.subs.has(s))
		if (missingWrites.length > 0 || missingSubs.length > 0) {
			out.push({ scriptPath, missingWrites, missingSubs })
		}
	}
	return out
}

/** One-line human summary for a toast; undefined when there is no drift. */
export function formatDrift(drift: DeployGraphDrift[]): string | undefined {
	if (drift.length === 0) return undefined
	return drift
		.map((d) => {
			const parts: string[] = []
			if (d.missingWrites.length > 0) {
				parts.push(`output ${d.missingWrites.join(', ')}`)
			}
			if (d.missingSubs.length > 0) {
				parts.push(`trigger on ${d.missingSubs.join(', ')}`)
			}
			return `${d.scriptPath}: ${parts.join(' and ')} shown in the editor did not persist at deploy`
		})
		.join('; ')
}
