import {
	GranularAclService,
	type AccessOtherAsset,
	type AccessReader,
	type AccessReads,
	type AccessReadsSource,
	type AccessReference
} from '$lib/gen'

export type RunnableKind = 'script' | 'flow' | 'app'

/** What an item reads when it runs, each read with who performs it. */
export type Needs = AccessReads

/** Unsaved editor content, scanned in place of the saved item. */
export type NeedsSource = AccessReadsSource & { kind?: RunnableKind }

/** Something access is granted on item by item, at the level its reads take. */
export type NeededItem = {
	kind: AccessReference['kind']
	path: string
	level: 'read' | 'write'
	sources: string[]
}

/** Something it reads that item grants do not cover (S3 objects, data tables, …). */
export type OtherAsset = Omit<AccessOtherAsset, 'reader'>

export type NeededAccess = { items: NeededItem[]; others: OtherAsset[] }

/**
 * What a script, flow or app reads when it runs, and who performs each read: the server scans
 * the item and everything it calls (`docs/access-reads.md`). `source` stands in for the saved
 * item, as unsaved editor content.
 */
export async function detectNeeds(
	workspace: string,
	kind: RunnableKind,
	path: string,
	source?: NeedsSource
): Promise<Needs> {
	return await GranularAclService.getAccessReads({
		workspace,
		requestBody: { kind, path: path || undefined, source }
	})
}

/**
 * What `principal` needs to run the item: what whoever opens it reads, what the item reads when
 * it runs as its caller or as `principal`, and what a runnable it reaches reads as `principal`.
 */
export function neededByGrantee(needs: Needs, principal: string): NeededAccess {
	const runsAsGrantee = needs.runs_as.type === 'caller' || needs.runs_as.principal === principal
	return select(
		needs,
		(reader) =>
			reader.type === 'viewer' ||
			(reader.type === 'root' && runsAsGrantee) ||
			(reader.type === 'account' && reader.principal === principal)
	)
}

/**
 * What `account` needs when the item runs as it: what the item reads, and what a runnable it
 * reaches reads as `account`. A full-code app's frontend reads as whoever opens it, so none of
 * that is listed.
 */
export function neededByRunAs(needs: Needs, account?: string): NeededAccess {
	return select(
		needs,
		(reader) =>
			reader.type === 'root' ||
			(account !== undefined && reader.type === 'account' && reader.principal === account)
	)
}

/** One row per item: a write is needed when any read writes, and running an item takes read. */
function select(needs: Needs, performs: (reader: AccessReader) => boolean): NeededAccess {
	const items = new Map<string, NeededItem>()
	for (const reference of needs.references) {
		if (!performs(reference.reader)) continue
		const key = `${reference.kind}:${reference.path}`
		const level = reference.access === 'write' ? 'write' : 'read'
		const item = items.get(key)
		if (item) {
			if (level === 'write') item.level = 'write'
			item.sources = [...new Set([...item.sources, ...reference.sources])]
		} else {
			items.set(key, {
				kind: reference.kind,
				path: reference.path,
				level,
				sources: reference.sources
			})
		}
	}
	const others = new Map<string, OtherAsset>()
	for (const { reader, ...asset } of needs.others) {
		if (!performs(reader)) continue
		const key = `${asset.kind}:${asset.path}`
		const other = others.get(key)
		others.set(
			key,
			other ? { ...other, sources: [...new Set([...other.sources, ...asset.sources])] } : asset
		)
	}
	return { items: [...items.values()], others: [...others.values()] }
}
