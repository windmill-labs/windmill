import { AppService, AssetService, FlowService, ScriptService, type AssetKind } from '$lib/gen'
import { inferAssets } from '$lib/infer'
import type { SupportedLanguage } from '$lib/common'

export type RunnableKind = 'script' | 'flow' | 'app'

/** Something a runnable reads that access is granted on, item by item. */
export type NeededItem = { kind: 'resource' | 'variable'; path: string; sources: string[] }

/** Something it reads that item grants do not cover (S3 objects, data tables, …). */
export type OtherAsset = { kind: AssetKind | string; path: string; sources: string[] }

export type Needs = { items: NeededItem[]; others: OtherAsset[]; errors: string[] }

const MAX_DEPTH = 4
const ITEM_PATH = /^[ufg]\/[^/]+\/.+/
const STATIC_REF = /^\$(res|var):(.+)$/
// The asset parser finds resources but has no variable kind, so literal variable reads are matched
// here. A path the code builds at run time is found by neither.
const VARIABLE_CALL = /\b(?:getVariable|get_variable)\(\s*["'`]([^"'`]+)["'`]/g

/**
 * What a script, flow or app reads when it runs, to grant to the account it runs as. The scripts
 * and flows it calls by path are followed for what they read, but need no grant themselves: the
 * runnable resolves them with its own access, not the account's.
 */
export async function detectNeeds(
	workspace: string,
	kind: RunnableKind,
	path: string
): Promise<Needs> {
	const items = new Map<string, NeededItem>()
	const others = new Map<string, OtherAsset>()
	const errors: string[] = []
	const visited = new Set<string>()

	function add(assetKind: string, assetPath: string, source: string) {
		const key = `${assetKind}:${assetPath}`
		const target =
			(assetKind === 'resource' || assetKind === 'variable') && ITEM_PATH.test(assetPath)
				? items
				: others
		const existing = target.get(key)
		if (existing) {
			if (!existing.sources.includes(source)) existing.sources.push(source)
		} else {
			target.set(key, { kind: assetKind as any, path: assetPath, sources: [source] })
		}
	}

	async function scanCode(language: string | undefined, content: string, source: string) {
		const result = await inferAssets(language as SupportedLanguage | undefined, content)
		if (result.status === 'ok') {
			for (const asset of result.assets) add(asset.kind, asset.path, source)
		}
		for (const match of content.matchAll(VARIABLE_CALL)) add('variable', match[1], source)
	}

	async function recorded(usageKind: 'script' | 'flow', usagePath: string, source: string) {
		const [assets] = await AssetService.listAssetsByUsage({
			workspace,
			requestBody: { usages: [{ kind: usageKind, path: usagePath }] }
		})
		for (const asset of (assets ?? []) as { kind: string; path: string }[]) {
			add(asset.kind, asset.path, source)
		}
	}

	/** Walks a flow or app definition for static references, inline code and calls by path. */
	async function walk(value: unknown, source: string, depth: number) {
		const pending: Promise<void>[] = []
		const visit = (node: unknown, label: string) => {
			if (typeof node === 'string') {
				const ref = STATIC_REF.exec(node)
				if (ref) add(ref[1] === 'res' ? 'resource' : 'variable', ref[2], label)
				return
			}
			if (Array.isArray(node)) {
				node.forEach((child) => visit(child, label))
				return
			}
			if (!node || typeof node !== 'object') return
			const obj = node as Record<string, unknown>
			const name =
				typeof obj.summary === 'string' && obj.summary
					? obj.summary
					: typeof obj.name === 'string'
						? obj.name
						: typeof obj.id === 'string'
							? obj.id
							: undefined
			const here = name ? `${source} › ${name}` : label
			if (typeof obj.content === 'string' && typeof obj.language === 'string') {
				pending.push(scanCode(obj.language, obj.content, here))
			}
			const calledKind =
				obj.runType === 'script' || obj.runType === 'flow'
					? obj.runType
					: obj.type === 'script' || obj.type === 'flow'
						? obj.type
						: undefined
			if (calledKind && typeof obj.path === 'string' && !obj.path.startsWith('hub/')) {
				pending.push(follow(calledKind, obj.path, depth + 1))
			}
			for (const [key, child] of Object.entries(obj)) {
				if (key !== 'content') visit(child, here)
			}
		}
		visit(value, source)
		await Promise.all(pending)
	}

	async function follow(runnableKind: RunnableKind, runnablePath: string, depth: number) {
		const key = `${runnableKind}:${runnablePath}`
		if (visited.has(key) || depth > MAX_DEPTH) return
		visited.add(key)
		try {
			if (runnableKind === 'script') {
				const script = await ScriptService.getScriptByPath({ workspace, path: runnablePath })
				await Promise.all([
					recorded('script', runnablePath, runnablePath),
					scanCode(script.language, script.content, runnablePath),
					walk(script.schema, runnablePath, depth)
				])
			} else if (runnableKind === 'flow') {
				const flow = await FlowService.getFlowByPath({ workspace, path: runnablePath })
				await Promise.all([
					recorded('flow', runnablePath, runnablePath),
					walk(flow.value, runnablePath, depth),
					walk(flow.schema, runnablePath, depth)
				])
			} else {
				// Apps record no asset usages, so their inline code is parsed here.
				const app = await AppService.getAppByPath({ workspace, path: runnablePath })
				await walk(app.value, runnablePath, depth)
			}
		} catch (e) {
			errors.push(`${runnablePath}: ${e?.body ?? e}`)
		}
	}

	await follow(kind, path, 0)
	const byPath = (a: { path: string }, b: { path: string }) => a.path.localeCompare(b.path)
	return {
		items: [...items.values()].sort(byPath),
		others: [...others.values()].sort(byPath),
		errors
	}
}
