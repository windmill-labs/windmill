/**
 * Point an already-imported project at a resource the workspace already has.
 *
 * The import writes `$res:f/<folder>/<name>` into every item that uses the project's
 * resource. This rewrites those references to an existing resource and deletes the stub,
 * so the project reads the workspace's own credential — the same end state the import
 * would have produced, reached after the fact.
 *
 * Two rules make that safe to run over deployed items:
 *
 *  - Nothing is written unless every referrer can be rewritten. A run that gets halfway
 *    leaves some items on the stub and some on the chosen resource, and deleting the stub
 *    then breaks the ones left behind. `planRetarget` answers first, `applyRetarget` acts.
 *  - Only items inside the project's folder are touched. The import wrote nothing outside
 *    it, so a reference from elsewhere is the user's own and not ours to rewrite.
 */

import { AppService, FlowService, ResourceService, ScheduleService, ScriptService } from '$lib/gen'
import {
	rewriteAppValue,
	rewriteContent,
	rewriteFlowValue,
	rewriteRawAppContent,
	rewriteTriggerConfig
} from '$lib/components/workspaceSettings/projectBundle'
import {
	TRIGGER_KINDS,
	WORKSPACE_TRIGGER_KINDS,
	type WorkspaceTriggerKind
} from '$lib/components/triggers/workspaceTriggersList'
import { updatePolicy } from '$lib/components/apps/editor/appPolicy'
import { updateRawAppPolicy } from '$lib/sharedUtils'
import type { App } from '$lib/components/apps/types'
import { apiErrorMessage as errorMessage } from '$lib/utils'

/**
 * The source files a raw app was imported from, keyed by the path it landed at.
 *
 * A raw app is deployed as a value plus a compiled bundle, and the bundle is stored where
 * the browser cannot read it back (`/apps/get_data/v/{id}` decrypts an embed secret). The
 * export already carries that bundle prebuilt as `/bundle.js` and `/bundle.css` — it is what
 * `installProject` uploaded in the first place — so a rewrite re-uploads it rather than
 * re-running the bundler.
 */
export type ExportedAppFiles = Record<string, Record<string, string>>

export type ReferrerKind = 'script' | 'flow' | 'app' | 'raw app' | 'trigger'

export interface Referrer {
	kind: ReferrerKind
	path: string
	/** Present for triggers: which kind's table it lives in. */
	triggerKind?: WorkspaceTriggerKind
}

export interface RetargetPlan {
	referrers: Referrer[]
	/**
	 * Referrers that cannot be rewritten, with the reason. Non-empty means the whole
	 * retarget is refused: see the all-or-nothing rule above.
	 */
	blocked: { path: string; reason: string }[]
}

export interface RetargetOutcome {
	rewritten: Referrer[]
	/** Set when a write failed. The stub is then left in place, so nothing is broken. */
	error?: string
	stubDeleted: boolean
}

/** Everything under the project's folder, so nothing outside it is ever rewritten. */
function inFolder(path: unknown, folder: string): boolean {
	return typeof path === 'string' && path.startsWith(`f/${folder}/`)
}

/**
 * Whether the deployed sources differ from the ones the export shipped. The two bundle
 * entries are excluded: `installProject` strips them out of `files` before it deploys, so
 * they are never part of the deployed set.
 */
function rawSourcesDiverged(
	deployed: Record<string, string>,
	exported: Record<string, string>
): boolean {
	const strip = (f: Record<string, string>) =>
		Object.fromEntries(
			Object.entries(f ?? {}).filter(([k]) => k !== '/bundle.js' && k !== '/bundle.css')
		)
	const a = strip(deployed)
	const b = strip(exported)
	const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort()
	return keys.some((k) => a[k] !== b[k])
}

/** Whether a serialized item mentions the resource, in either reference spelling. */
function mentions(blob: string, resourcePath: string): boolean {
	return blob.includes(`$res:${resourcePath}`) || blob.includes(`res://${resourcePath}`)
}

/**
 * Which deployed items reference the stub, and whether each can be rewritten.
 *
 * The `listSearch*` endpoints return each item's content in one call per kind, so this is
 * four calls plus one per trigger kind rather than one per item.
 */
export async function planRetarget(
	workspace: string,
	folder: string,
	from: string,
	opts: { hasEeLicense: boolean; exportedAppFiles?: ExportedAppFiles }
): Promise<RetargetPlan> {
	const referrers: Referrer[] = []
	const blocked: { path: string; reason: string }[] = []

	const [scripts, flows, apps] = await Promise.all([
		ScriptService.listSearchScript({ workspace }),
		FlowService.listSearchFlow({ workspace }),
		AppService.listSearchApp({ workspace })
	])

	for (const s of scripts ?? []) {
		if (!inFolder(s.path, folder)) continue
		if (mentions(String(s.content ?? ''), from)) referrers.push({ kind: 'script', path: s.path! })
	}
	for (const f of flows ?? []) {
		if (!inFolder(f.path, folder)) continue
		if (mentions(JSON.stringify(f.value ?? {}), from))
			referrers.push({ kind: 'flow', path: f.path! })
	}
	for (const a of apps ?? []) {
		if (!inFolder(a.path, folder)) continue
		const value: any = a.value ?? {}
		if (!mentions(JSON.stringify(value), from)) continue
		// `files` + `runnables` and no `grid` is the deployed shape of a raw app; the
		// low-code one keeps its components under `grid`.
		const isRaw = !!value.files && !!value.runnables
		if (!isRaw) {
			referrers.push({ kind: 'app', path: a.path! })
			continue
		}
		const exported = opts.exportedAppFiles?.[a.path!]
		if (!exported) {
			blocked.push({ path: a.path!, reason: 'is a raw app the export does not describe' })
			continue
		}
		// The bundle about to be re-uploaded was built from the export's sources. If the
		// deployed sources have moved on, it is behind them, and uploading it would revert
		// whatever was changed since the import.
		if (rawSourcesDiverged(value.files ?? {}, exported)) {
			blocked.push({ path: a.path!, reason: 'has been edited since the import' })
			continue
		}
		referrers.push({ kind: 'raw app', path: a.path! })
	}

	for (const kind of WORKSPACE_TRIGGER_KINDS) {
		const def = TRIGGER_KINDS[kind]
		if (def.eeOnly && !opts.hasEeLicense) continue
		let rows: Array<Record<string, any>> = []
		try {
			rows = await def.list(workspace)
		} catch {
			// A kind that cannot be listed cannot be cleared either: refusing is the
			// all-or-nothing rule, since a trigger left on the stub breaks when it is deleted.
			blocked.push({ path: `${def.badge} triggers`, reason: 'could not be listed' })
			continue
		}
		for (const t of rows) {
			if (!inFolder(t.path, folder)) continue
			if (!mentions(JSON.stringify(t), from)) continue
			// `schedule` has no `update` in the table because its service takes a different body
			// shape; `rewriteTrigger` handles it directly, the way the import's create does.
			if (kind !== 'schedule' && !def.update) {
				blocked.push({
					path: String(t.path),
					reason: `${def.badge} triggers cannot be updated from here`
				})
				continue
			}
			referrers.push({ kind: 'trigger', path: String(t.path), triggerKind: kind })
		}
	}

	return { referrers, blocked }
}

/**
 * Rewrite every referrer and delete the stub.
 *
 * Refuses outright when `planRetarget` reports anything blocked. The first write failure
 * stops the run and leaves the stub in place: the items already rewritten point at the
 * chosen resource, the rest still point at the stub, and both resolve.
 */
export async function applyRetarget(args: {
	workspace: string
	folder: string
	from: string
	to: string
	hasEeLicense: boolean
	/** Raw-app sources from the export, retargeted to the folder. See `ExportedAppFiles`. */
	exportedAppFiles?: ExportedAppFiles
}): Promise<RetargetOutcome> {
	const { workspace, folder, from, to, hasEeLicense, exportedAppFiles } = args
	const map = new Map([[from, to]])
	const rewritten: Referrer[] = []

	const plan = await planRetarget(workspace, folder, from, { hasEeLicense, exportedAppFiles })
	if (plan.blocked.length > 0) {
		const first = plan.blocked[0]
		return {
			rewritten,
			stubDeleted: false,
			error: `${first.path} ${first.reason} — nothing was changed`
		}
	}

	try {
		for (const r of plan.referrers) {
			if (r.kind === 'script') await rewriteScript(workspace, r.path, map)
			else if (r.kind === 'flow') await rewriteFlow(workspace, r.path, map)
			else if (r.kind === 'app') await rewriteApp(workspace, r.path, map)
			else if (r.kind === 'raw app')
				await rewriteRawApp(workspace, r.path, map, exportedAppFiles?.[r.path] ?? {})
			else await rewriteTrigger(workspace, r, map)
			rewritten.push(r)
		}
	} catch (e: any) {
		return { rewritten, stubDeleted: false, error: errorMessage(e) }
	}

	// Last, and only once every referrer is off it: the stub is what keeps a reference this
	// missed resolving, so it is the one thing that must not go early.
	try {
		await ResourceService.deleteResource({ workspace, path: from })
	} catch (e: any) {
		return { rewritten, stubDeleted: false, error: errorMessage(e) }
	}
	return { rewritten, stubDeleted: true }
}

/**
 * A new script version, the way the editor saves one. Spread rather than field-by-field:
 * `Script` and `NewScript` share their names, and listing them here would silently drop
 * whichever field someone adds next.
 */
async function rewriteScript(workspace: string, path: string, map: Map<string, string>) {
	const s: any = await ScriptService.getScriptByPath({ workspace, path })
	const content = rewriteContent(s.content ?? '', map)
	if (content === s.content) return
	await ScriptService.createScript({
		workspace,
		requestBody: { ...s, content, parent_hash: s.hash, deployment_message: undefined }
	})
}

async function rewriteFlow(workspace: string, path: string, map: Map<string, string>) {
	const f: any = await FlowService.getFlowByPath({ workspace, path })
	await FlowService.updateFlow({
		workspace,
		path,
		requestBody: { ...f, path, value: rewriteFlowValue(f.value, map) }
	})
}

/**
 * The policy is recomputed rather than carried over: `triggerables_v2` is keyed by
 * `<component>:rawscript/<sha256(inline content)>`, and rewriting an inline runnable's
 * content changes that key — a copied policy would leave it "forbidden by policy".
 * Mirrors what `installProject` does on import.
 */
async function rewriteApp(workspace: string, path: string, map: Map<string, string>) {
	const a: any = await AppService.getAppByPath({ workspace, path })
	const next = rewriteAppValue(a.value ?? {}, map)
	const policy = (await updatePolicy(next as App, undefined)) as any
	if (!policy.execution_mode) policy.execution_mode = 'publisher'
	await AppService.updateApp({ workspace, path, requestBody: { path, value: next, policy } })
}

/**
 * A raw app: its deployed value carries the sources and the runnables, and `updateAppRaw`
 * refuses without a bundle. The bundle comes from the export rather than the bundler — it is
 * the one the import uploaded, and `planRetarget` has already refused if the deployed
 * sources have moved on from it.
 */
async function rewriteRawApp(
	workspace: string,
	path: string,
	map: Map<string, string>,
	exportedFiles: Record<string, string>
) {
	const a: any = await AppService.getAppByPath({ workspace, path })
	const value: any = a.value ?? {}
	// One walk over the whole value: `$res:` tokens live in the runnables and can appear in
	// the sources too, and both are plain text inside this JSON.
	const next = JSON.parse(rewriteRawAppContent(JSON.stringify(value), map))
	const runnables = next.runnables ?? {}
	const policy = (await updateRawAppPolicy(runnables, a.policy)) as any
	if (!policy.execution_mode) policy.execution_mode = 'publisher'
	const files = { ...(next.files ?? {}) }
	delete files['/bundle.js']
	delete files['/bundle.css']
	await AppService.updateAppRaw({
		workspace,
		path,
		formData: {
			app: {
				path,
				summary: a.summary ?? '',
				value: {
					files,
					runnables,
					...(next.data !== undefined ? { data: next.data } : {}),
					...(next.datatables !== undefined ? { datatables: next.datatables } : {})
				},
				policy
			},
			js: exportedFiles['/bundle.js'] ?? '',
			css: exportedFiles['/bundle.css'] ?? ''
		}
	})
}

/**
 * The trigger's own row, rewritten and written back. `enabled` is deliberately not sent:
 * imported triggers are created disabled and re-enabling one is the user's decision, not a
 * side effect of pointing it at a credential.
 */
async function rewriteTrigger(workspace: string, r: Referrer, map: Map<string, string>) {
	const def = TRIGGER_KINDS[r.triggerKind!]
	const rows = await def.list(workspace)
	const row: any = rows.find((t) => t.path === r.path)
	if (!row) throw new Error(`trigger ${r.path} is no longer there`)
	const { enabled: _enabled, ...rest } = rewriteTriggerConfig(row, map) as any
	if (r.triggerKind === 'schedule') {
		// `EditSchedule` needs these three; everything else on the row carries over by name.
		await ScheduleService.updateSchedule({
			workspace,
			path: r.path,
			requestBody: {
				...rest,
				schedule: rest.schedule ?? '0 0 * * * *',
				timezone: rest.timezone ?? 'UTC',
				args: rest.args ?? {}
			}
		})
		return
	}
	await def.update!(workspace, r.path, rest)
}
