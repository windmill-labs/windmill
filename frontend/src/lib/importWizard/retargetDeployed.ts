/**
 * Point an already-imported project at a resource the workspace already has.
 *
 * The import writes `$res:f/<folder>/<name>` into every item that uses the project's
 * resource — except a trigger, which holds the bare path in its own `*_resource_path`
 * field. This rewrites those references to an existing resource, so the project reads the
 * workspace's own credential — the same end state the import would have produced, reached
 * after the fact.
 *
 * Two rules make that safe to run over deployed items:
 *
 *  - Only items inside the project's folder are rewritten. The import wrote nothing outside
 *    it, so a reference from elsewhere is the user's own and not ours to move.
 *  - The stub is deleted only when the scan can prove it saw every reference to it.
 *    Listings come back capped, a trigger kind can fail to list, and a reference can sit
 *    where no rewriter reaches — each of those is a gap, and any gap keeps the stub.
 *
 * Rewriting is separable from deleting, and only the delete is destructive. An item moved
 * onto the chosen resource resolves whether or not the stub survives; an item the scan never
 * saw resolves only while the stub is there. So an incomplete scan downgrades the run to
 * "rewritten, stub kept" rather than refusing it — the outcome names the gaps so the caller
 * can say the placeholder is still around.
 */

import { AppService, FlowService, ResourceService, ScheduleService, ScriptService } from '$lib/gen'
import {
	rewriteAppValue,
	rewriteContent,
	rewriteFlowValue,
	rewriteRawAppContent,
	rewriteTriggerConfig,
	referencesResourcePath,
	holdsResourceToken,
	textHoldsBarePath
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

export type ReferrerKind = 'script' | 'flow' | 'app' | 'raw app' | 'trigger'

export interface Referrer {
	kind: ReferrerKind
	path: string
	/** Present for triggers: which kind's table it lives in. */
	triggerKind?: WorkspaceTriggerKind
	/**
	 * Present for triggers: the row the scan read, which is also what the write sends back.
	 * Re-reading it costs another listing of the whole kind per trigger — and for schedules a
	 * listing plus a detail fetch per row, since `list` resolves each one.
	 */
	row?: Record<string, any>
}

/** Something the scan could not account for. `path` names an item, or a listing standing in
 *  for every item it failed to return. */
export interface Gap {
	path: string
	reason: string
}

export interface RetargetPlan {
	/** Items whose reference this run will move. */
	referrers: Referrer[]
	/**
	 * Why the scan cannot claim it saw every reference to the stub. Empty is the only state
	 * in which deleting the stub is provably safe.
	 */
	gaps: Gap[]
}

export interface RetargetOutcome {
	/** Items now reading the chosen resource. */
	rewritten: Referrer[]
	/** Why the stub was kept, when it was. */
	gaps: Gap[]
	stubDeleted: boolean
	/** Set when a write failed. The run stopped there and the stub stays, so what was
	 *  rewritten before it and what was not both resolve. */
	error?: string
}

/** Everything under the project's folder, which is all this rewrites. */
function inFolder(path: unknown, folder: string): boolean {
	return typeof path === 'string' && path.startsWith(`f/${folder}/`)
}

/**
 * Whether the item names the resource path anywhere a rewriter would not reach it.
 *
 * A script, flow or app spells a reference the rewriters move as a `$res:` token and nothing
 * else. The path appearing any other way is either a reference beyond them — a static string
 * value in a component, an argument inline code assembles itself — or not the resource at
 * all, such as a step running a script that happens to share the path. Neither is rewritable:
 * moving the first is beyond the rewriters, and moving the second would repoint a runnable at
 * a credential. Both are reasons the stub has to outlive the run.
 *
 * The whole serialized item is searched, because inline code is a string inside it and a path
 * written in code is no less a reference for being surrounded by other characters. Triggers
 * are the exception and hold the bare path by design, so they never come here.
 */
function namesPathUnreachably(value: unknown, path: string): boolean {
	return textHoldsBarePath(JSON.stringify(value ?? null), path)
}

/**
 * What each `listSearch*` endpoint caps its answer at, server-side. The queries carry no
 * `ORDER BY` and the routes take no pagination, so a full page is an arbitrary subset with no
 * page two to ask for.
 *
 * A full page is a sound truncation test only for an unscoped caller, which a wizard session
 * is. The server applies its scope-path predicate to the rows the `LIMIT` already returned,
 * so a scoped token can be handed a short page cut from a truncated query — reuse this scan
 * under one and the cap goes undetected.
 */
const SEARCH_LIMITS = { script: 10000, flow: 1000, app: 1000 }

/**
 * What a trigger listing caps at. The kinds' list routes take pagination this table does not
 * pass, so each answers with the server's `DEFAULT_PER_PAGE`. A full page is read the same
 * way as a full `listSearch*` page: as a listing that cannot account for the rest.
 */
const TRIGGER_LIST_LIMIT = 1000

/** A reference this run will not move, recorded so the stub outlives it. */
const OUTSIDE_PROJECT = 'reads this resource from outside the project'
const UNREACHABLE_REFERENCE = 'names the resource path outside a $res: reference'

/**
 * Which deployed items reference the stub, which of them this run can move, and what it
 * could not account for.
 *
 * The `listSearch*` endpoints return each item's content in one call per kind, so this is
 * three calls plus one per trigger kind rather than one per item.
 */
export async function planRetarget(
	workspace: string,
	folder: string,
	from: string,
	opts: { hasEeLicense: boolean }
): Promise<RetargetPlan> {
	const referrers: Referrer[] = []
	const gaps: Gap[] = []

	const [scripts, flows, apps] = await Promise.all([
		ScriptService.listSearchScript({ workspace }),
		FlowService.listSearchFlow({ workspace }),
		AppService.listSearchApp({ workspace })
	])

	for (const [label, rows, limit] of [
		['Scripts', scripts, SEARCH_LIMITS.script],
		['Flows', flows, SEARCH_LIMITS.flow],
		['Apps', apps, SEARCH_LIMITS.app]
	] as const) {
		if ((rows?.length ?? 0) >= limit) gaps.push({ path: label, reason: 'could not all be listed' })
	}

	// The listings are workspace-wide, so an item outside the folder is seen for free. It is
	// the user's own and stays on the stub, which is the whole reason the stub stays too.
	for (const s of scripts ?? []) {
		const content = String(s.content ?? '')
		const reads = referencesResourcePath(content, from)
		// `rewriteContent` moves the `$res:` tokens and nothing else, so a path the code spells
		// out is a reference this run leaves behind.
		const bare = textHoldsBarePath(content, from)
		if (!reads && !bare) continue
		if (!inFolder(s.path, folder)) {
			gaps.push({ path: s.path!, reason: OUTSIDE_PROJECT })
			continue
		}
		if (bare) {
			gaps.push({ path: s.path!, reason: UNREACHABLE_REFERENCE })
			if (!reads) continue
		}
		referrers.push({ kind: 'script', path: s.path! })
	}
	for (const f of flows ?? []) {
		const value: any = f.value ?? {}
		// A token is the only spelling a rewriter moves. `referencesResourcePath` would also
		// count a whole string equal to the path, which is the unreachable case `bare` covers.
		const reads = holdsResourceToken(value, from)
		const bare = namesPathUnreachably(value, from)
		if (!reads && !bare) continue
		if (!inFolder(f.path, folder)) {
			gaps.push({ path: f.path!, reason: OUTSIDE_PROJECT })
			continue
		}
		if (bare) {
			gaps.push({ path: f.path!, reason: UNREACHABLE_REFERENCE })
			if (!reads) continue
		}
		referrers.push({ kind: 'flow', path: f.path! })
	}
	for (const a of apps ?? []) {
		const value: any = a.value ?? {}
		// A token is the only spelling a rewriter moves. `referencesResourcePath` would also
		// count a whole string equal to the path, which is the unreachable case `bare` covers.
		const reads = holdsResourceToken(value, from)
		const bare = namesPathUnreachably(value, from)
		if (!reads && !bare) continue
		if (!inFolder(a.path, folder)) {
			gaps.push({ path: a.path!, reason: OUTSIDE_PROJECT })
			continue
		}
		if (bare) {
			gaps.push({ path: a.path!, reason: UNREACHABLE_REFERENCE })
			if (!reads) continue
		}
		// `files` + `runnables` and no `grid` is the deployed shape of a raw app; the
		// low-code one keeps its components under `grid`.
		const isRaw = !!value.files && !!value.runnables
		referrers.push({ kind: isRaw ? 'raw app' : 'app', path: a.path! })
	}

	for (const kind of WORKSPACE_TRIGGER_KINDS) {
		const def = TRIGGER_KINDS[kind]
		if (def.eeOnly && !opts.hasEeLicense) continue
		let rows: Array<Record<string, any>> = []
		// A 404 is not an incomplete listing — the instance has that trigger feature compiled
		// out, so there is no trigger of the kind to have missed. Anything else means triggers
		// of this kind may reference the stub without this ever seeing them.
		let incomplete = false
		try {
			rows = await def.list(workspace, () => (incomplete = true))
		} catch (e: any) {
			if (e?.status === 404) continue
			incomplete = true
		}
		if (incomplete) {
			gaps.push({ path: `${def.badge} triggers`, reason: 'could not be listed' })
			continue
		}
		if (rows.length >= TRIGGER_LIST_LIMIT) {
			gaps.push({ path: `${def.badge} triggers`, reason: 'could not all be listed' })
			continue
		}
		for (const t of rows) {
			if (!referencesResourcePath(t, from)) continue
			if (!inFolder(t.path, folder)) {
				gaps.push({ path: String(t.path), reason: OUTSIDE_PROJECT })
				continue
			}
			// `schedule` has no `update` in the table because its service takes a different body
			// shape; `rewriteTrigger` handles it directly, the way the import's create does.
			if (kind !== 'schedule' && !def.update) {
				gaps.push({
					path: String(t.path),
					reason: `${def.badge} triggers cannot be updated from here`
				})
				continue
			}
			referrers.push({ kind: 'trigger', path: String(t.path), triggerKind: kind, row: t })
		}
	}

	return { referrers, gaps }
}

/**
 * Rewrite every referrer the plan found, then delete the stub if the plan came back clean.
 *
 * A gap never stops the rewriting — moving an item onto the chosen resource is safe on its
 * own. It stops only the delete, which is the one step that can strand a reference nobody
 * looked at.
 */
export async function applyRetarget(args: {
	workspace: string
	folder: string
	from: string
	to: string
	hasEeLicense: boolean
}): Promise<RetargetOutcome> {
	const { workspace, folder, from, to, hasEeLicense } = args
	const map = new Map([[from, to]])
	const rewritten: Referrer[] = []

	const plan = await planRetarget(workspace, folder, from, { hasEeLicense })
	const gaps = [...plan.gaps]

	for (const r of plan.referrers) {
		let moved: true | string
		try {
			if (r.kind === 'script') moved = await rewriteScript(workspace, r.path, map)
			else if (r.kind === 'flow') moved = await rewriteFlow(workspace, r.path, map)
			else if (r.kind === 'app') moved = await rewriteApp(workspace, r.path, map)
			else if (r.kind === 'raw app') moved = await rewriteRawApp(workspace, r.path, map)
			else moved = await rewriteTrigger(workspace, r, map)
		} catch (e: any) {
			return { rewritten, gaps, stubDeleted: false, error: errorMessage(e) }
		}
		if (moved === true) rewritten.push(r)
		else gaps.push({ path: r.path, reason: moved })
	}

	if (gaps.length > 0) return { rewritten, gaps, stubDeleted: false }

	try {
		await ResourceService.deleteResource({ workspace, path: from })
	} catch (e: any) {
		return { rewritten, gaps, stubDeleted: false, error: errorMessage(e) }
	}
	return { rewritten, gaps, stubDeleted: true }
}

/** Why a rewriter left an item where it was, or `true` when it moved it. */
const CHANGED_UNDER_US = 'changed while it was being retargeted'

/**
 * Whether the rewrite moved every `$res:` token it was there to move.
 *
 * Only tokens: a path the item also spells out unreachably is recorded as a gap by the plan,
 * and re-reading it here would report the same item twice — once as unmovable and once as
 * having changed underfoot. So a `false` here means what it says, that the item's tokens are
 * not where the rewrite should have put them, which is a change between the plan and the
 * write. Each rewriter answers with this rather than writing.
 */
function relocated(next: unknown, map: Map<string, string>): boolean {
	for (const from of map.keys()) if (holdsResourceToken(next, from)) return false
	return true
}

/**
 * Every write here is an in-place edit of a deployed item, not a redeployment by whoever
 * opened the wizard, so each one has to say so.
 *
 * `preserve_on_behalf_of` is the flag that says it. Without it the backend replaces the
 * item's stored run identity with the caller's — `resolve_on_behalf_of` for scripts and
 * flows, the `should_preserve` branch of `update_app` for apps — and an imported item that
 * ran as a service account would silently start running as the person who picked a
 * credential. The backend still gates it on `wm_deployers` membership, so a caller who
 * cannot preserve gets what they would have got anyway.
 *
 * For apps the policy is the other half: it carries the run identity, the execution mode and
 * the sandbox rules, so it is read from the deployed app and handed back to the recompute
 * rather than rebuilt from nothing.
 */
const PRESERVE_DEPLOYED_IDENTITY = { preserve_on_behalf_of: true }

/**
 * A new script version, the way the editor saves one. Spread rather than field-by-field:
 * `Script` and `NewScript` share their names, and listing them here would silently drop
 * whichever field someone adds next.
 */
async function rewriteScript(
	workspace: string,
	path: string,
	map: Map<string, string>
): Promise<true | string> {
	const s: any = await ScriptService.getScriptByPath({ workspace, path })
	const content = rewriteContent(s.content ?? '', map)
	if (!relocated(content, map)) return CHANGED_UNDER_US
	if (content === s.content) return true
	await ScriptService.createScript({
		workspace,
		requestBody: {
			...s,
			...PRESERVE_DEPLOYED_IDENTITY,
			content,
			parent_hash: s.hash,
			deployment_message: undefined
		}
	})
	return true
}

async function rewriteFlow(
	workspace: string,
	path: string,
	map: Map<string, string>
): Promise<true | string> {
	const f: any = await FlowService.getFlowByPath({ workspace, path })
	const value = rewriteFlowValue(f.value, map)
	if (!relocated(value, map)) return CHANGED_UNDER_US
	// Nothing moved: the item was listed for a reference no rewriter reaches, already gapped.
	if (JSON.stringify(value) === JSON.stringify(f.value)) return true
	await FlowService.updateFlow({
		workspace,
		path,
		requestBody: { ...f, ...PRESERVE_DEPLOYED_IDENTITY, path, value }
	})
	return true
}

/**
 * The deployed policy is recomputed, not rebuilt. Recomputing is required: `triggerables_v2`
 * is keyed by `<component>:rawscript/<sha256(inline content)>`, and rewriting an inline
 * runnable's content changes that key, so a policy copied verbatim would leave the component
 * "forbidden by policy". Handing the deployed policy to the recompute is what keeps the run
 * identity, the sandbox rules and everything else it does not touch.
 *
 * No execution mode is defaulted. The backend keeps the deployed mode when the submitted
 * policy states none, and stating one here would put a `viewer` app on the publisher's
 * identity.
 */
async function rewriteApp(
	workspace: string,
	path: string,
	map: Map<string, string>
): Promise<true | string> {
	const a: any = await AppService.getAppByPath({ workspace, path })
	const next = rewriteAppValue(a.value ?? {}, map)
	if (!relocated(next, map)) return CHANGED_UNDER_US
	// Nothing moved: the item was listed for a reference no rewriter reaches, already gapped.
	if (JSON.stringify(next) === JSON.stringify(a.value ?? {})) return true
	const policy = (await updatePolicy(next as App, a.policy)) as any
	await AppService.updateApp({
		workspace,
		path,
		requestBody: { ...PRESERVE_DEPLOYED_IDENTITY, path, value: next, policy }
	})
	return true
}

/**
 * One half of a deployed raw app's compiled bundle, read back the way the Hub publish reads
 * it: `/apps/get_data/v/{secret}.{ext}` serves it to anyone holding the secret, and the
 * secret is minted for the caller against a plain `apps:read:<path>` check.
 *
 * A missing `.css` is an app that ships no styles. A missing `.js` is a broken deployment,
 * and uploading an empty one in its place would break it further.
 */
async function fetchBundlePart(
	workspace: string,
	secret: string,
	ext: 'js' | 'css'
): Promise<string> {
	const res = await fetch(
		`/api/w/${encodeURIComponent(workspace)}/apps/get_data/v/${secret}.${ext}`,
		{ credentials: 'include' }
	)
	if (res.ok) return await res.text()
	if (ext === 'css' && res.status === 404) return ''
	throw new Error(`the compiled bundle could not be read (${res.status})`)
}

/**
 * A raw app: its deployed value carries the sources and the runnables, and `updateAppRaw`
 * refuses without a bundle. The bundle is the deployed one, read back, rewritten and sent
 * again — rebuilding it is not possible from the browser and is not needed, but re-uploading
 * it untouched is not an option either.
 *
 * The bundle is compiled from the sources, so a `$res:` a source file spells out is baked
 * into it. The import rewrites that copy — `retargetProjectExport` runs while `/bundle.js`
 * is still one of `files`, and only `installProject` splits it out afterwards. Sending the
 * deployed bundle back unrewritten would undo on reuse what the import got right, and the
 * app would keep reading a resource this run is about to delete.
 */
async function rewriteRawApp(
	workspace: string,
	path: string,
	map: Map<string, string>
): Promise<true | string> {
	const a: any = await AppService.getAppByPath({ workspace, path })
	const value: any = a.value ?? {}
	// One walk over the whole value: `$res:` tokens live in the runnables and can appear in
	// the sources too, and both are plain text inside this JSON.
	const next = JSON.parse(rewriteRawAppContent(JSON.stringify(value), map))
	if (!relocated(next, map)) return CHANGED_UNDER_US
	const runnables = next.runnables ?? {}
	const policy = (await updateRawAppPolicy(runnables, a.policy)) as any
	const secret = await AppService.getPublicSecretOfLatestVersionOfApp({ workspace, path })
	const [deployedJs, deployedCss] = await Promise.all([
		fetchBundlePart(workspace, secret, 'js'),
		fetchBundlePart(workspace, secret, 'css')
	])
	const js = rewriteContent(deployedJs, map)
	const css = rewriteContent(deployedCss, map)
	// `rewriteContent` moves the `$res:` tokens. A path the bundle spells out any other way
	// is one nothing here can move, and uploading it would leave the app reading a resource
	// about to be deleted.
	for (const stub of map.keys()) if (textHoldsBarePath(js, stub)) return UNREACHABLE_REFERENCE
	// Nothing moved: the app was listed for a reference no rewriter reaches, already gapped.
	// Uploading an unchanged app would cut it a version that differs from the last in nothing.
	if (js === deployedJs && css === deployedCss && JSON.stringify(next) === JSON.stringify(value))
		return true
	const files = { ...(next.files ?? {}) }
	delete files['/bundle.js']
	delete files['/bundle.css']
	await AppService.updateAppRaw({
		workspace,
		path,
		formData: {
			app: {
				...PRESERVE_DEPLOYED_IDENTITY,
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
			js,
			css
		}
	})
	return true
}

/**
 * The trigger's own row, rewritten and written back. `enabled` is deliberately not sent:
 * imported triggers are created disabled and re-enabling one is the user's decision, not a
 * side effect of pointing it at a credential.
 *
 * `path` and `script_path` are put back from the row afterwards. The rewrite remaps any
 * string equal to the stub's path, so a trigger sitting at the path the resource used to
 * hold — or running a script that does — would otherwise be renamed, or repointed at the
 * reused resource, along with the reference.
 *
 * A trigger states its run identity as `permissioned_as`, not the `on_behalf_of` the other
 * kinds use, and `resolve_permissioned_as` keeps the row's value only when
 * `preserve_permissioned_as` says so. Without the pair a trigger created under a folder's
 * `default_permissioned_as` would start running as whoever picked the credential.
 */
async function rewriteTrigger(
	workspace: string,
	r: Referrer,
	map: Map<string, string>
): Promise<true | string> {
	const def = TRIGGER_KINDS[r.triggerKind!]
	const row: any = r.row ?? {}
	const { enabled: _enabled, ...rest } = {
		...(rewriteTriggerConfig(row, map) as any),
		path: r.path,
		...(typeof row.script_path === 'string' ? { script_path: row.script_path } : {}),
		...(typeof row.permissioned_as === 'string'
			? { permissioned_as: row.permissioned_as, preserve_permissioned_as: true }
			: {})
	}
	// No `relocated` check: `rewriteTriggerConfig` remaps every bare match at every depth, so
	// the resource field always moves. What can still read `from` afterwards is a restored
	// identity field, which is not a reference to the resource at all.
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
		return true
	}
	await def.update!(workspace, r.path, rest)
	return true
}
