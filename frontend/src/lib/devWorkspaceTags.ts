import { derived, get, writable } from 'svelte/store'
import { WorkerService } from '$lib/gen'

/** Mirrors `DEFAULT_DEV_WORKSPACE_TAG_SUFFIX` in `windmill-common/src/worker.rs`. */
export const DEFAULT_DEV_WORKSPACE_TAG_SUFFIX = '-dev'
const WM_FORK_PREFIX = 'wm-fork-'

/** Mirrors `is_valid_dev_workspace_tag_suffix`: the suffix ends up in SQL literals and tag lists. */
export const DEV_WORKSPACE_TAG_SUFFIX_PATTERN = /^[A-Za-z0-9_.-]{1,32}$/

export type DevWorkspaceIsolationState = {
	enabled: boolean
	suffix: string
	/** Forks of a workspace with no dev workspace in its lineage run on their parent's workers. */
	skip_non_dev_forks: boolean
}

/** `undefined` until loaded. */
export const devWorkspaceIsolation = writable<DevWorkspaceIsolationState | undefined>(undefined)
/** The suffix new dev workspace and fork jobs get: `null` while isolation is off. */
export const devWorkspaceTagSuffix = derived(devWorkspaceIsolation, (i) =>
	i === undefined ? undefined : i.enabled ? i.suffix : null
)
/** Whether forks with no dev workspace in their lineage run on their parent's workers. */
export const devWorkspaceSkipNonDevForks = derived(
	devWorkspaceIsolation,
	(i) => i?.skip_non_dev_forks ?? false
)
/** The suffix worker groups serving dev workspaces listen with, isolation on or off. */
export const devWorkspaceListenSuffix = derived(
	devWorkspaceIsolation,
	(i) => i?.suffix ?? DEFAULT_DEV_WORKSPACE_TAG_SUFFIX
)

let inflight: Promise<void> | undefined
export function loadDevWorkspaceTagSuffix(force = false): Promise<void> {
	if (!force && get(devWorkspaceIsolation) !== undefined) return Promise.resolve()
	if (!force && inflight) return inflight
	// Started inside the chain so any failure lands in the catch: callers on a job's error path
	// await this and must not fail because the setting could not be read.
	inflight = Promise.resolve()
		.then(() => WorkerService.getDevWorkspaceIsolation())
		.then((i) => devWorkspaceIsolation.set(i))
		.catch(() =>
			devWorkspaceIsolation.set({
				enabled: false,
				suffix: DEFAULT_DEV_WORKSPACE_TAG_SUFFIX,
				skip_non_dev_forks: false
			})
		)
		.finally(() => (inflight = undefined))
	return inflight
}

/** Mirrors `with_dev_workspace_suffix`: idempotent, as an inherited tag already carries it. */
export function withDevSuffix(tag: string, suffix: string): string {
	return tag.endsWith(suffix) ? tag : `${tag}${suffix}`
}

/** The suffix `tag` carries as a dev workspace job's tag, if isolation is on and it does. */
export function devSuffixOf(tag: string | undefined): string | undefined {
	const suffix = get(devWorkspaceTagSuffix)
	return suffix && tag?.endsWith(suffix) ? suffix : undefined
}

/** What to do about a dev workspace tag no worker serves, empty for any other tag. */
export function devTagNoWorkerHint(tag: string): string {
	const suffix = devSuffixOf(tag)
	if (!suffix) return ''
	return (
		` "${tag}" ends with "${suffix}", the suffix of jobs of dev workspaces and forks: only worker ` +
		`groups flagged as serving dev workspaces listen to it. Flag a worker group as serving dev ` +
		`workspaces (Workers page) whose tags include "${tag.slice(0, -suffix.length)}".`
	)
}

type LineageWorkspace = {
	id: string
	parent_workspace_id?: string | null
	is_dev_workspace?: boolean
}

/** Whether a workspace has a parent, or is (or was) a fork. */
export function isDevOrForkWorkspace(w: LineageWorkspace): boolean {
	return !!w.parent_workspace_id || w.id.startsWith(WM_FORK_PREFIX)
}

/** Mirrors `in_dev_lineage` in `windmill-queue/src/tags.rs`: a dev workspace on the ancestor chain,
 * `w` included, an orphaned fork counting as one. The server reads the whole chain; here an
 * ancestor missing from `workspaces` (one the user is not a member of) also counts as one, so a
 * hint errs toward the dev workers rather than claiming a job reaches the parent's. */
function isInDevLineage(w: LineageWorkspace, workspaces: LineageWorkspace[]): boolean {
	let current: LineageWorkspace | undefined = w
	for (let depth = 0; current && depth <= 20; depth++) {
		if (current.is_dev_workspace) return true
		const parentId = current.parent_workspace_id
		if (!parentId) return current.id.startsWith(WM_FORK_PREFIX)
		current = workspaces.find((x) => x.id === parentId)
	}
	return true
}

/** Whether a workspace's jobs run on dev workers while isolation is on. Mirrors
 * `is_isolated_workspace` in `windmill-common/src/worker.rs`. */
export function isIsolatedWorkspace(
	w: LineageWorkspace,
	workspaces: LineageWorkspace[],
	skipNonDevForks: boolean
): boolean {
	return isDevOrForkWorkspace(w) && (!skipNonDevForks || isInDevLineage(w, workspaces))
}

/** The same check for a workspace id, looked up in `workspaces` (pass the store's value so a
 * reactive caller recomputes once the list loads). */
export function isIsolatedWorkspaceId(
	workspaceId: string | undefined,
	workspaces: LineageWorkspace[],
	skipNonDevForks: boolean
): boolean {
	if (!workspaceId) return false
	return isIsolatedWorkspace(
		workspaces.find((w) => w.id === workspaceId) ?? { id: workspaceId },
		workspaces,
		skipNonDevForks
	)
}

/** Whether a new throwaway fork of `baseId` would run on dev workers while isolation is on. */
export function isForkOfIsolated(
	baseId: string,
	workspaces: LineageWorkspace[],
	skipNonDevForks: boolean
): boolean {
	return isIsolatedWorkspace(
		{ id: `${WM_FORK_PREFIX}new`, parent_workspace_id: baseId, is_dev_workspace: false },
		workspaces,
		skipNonDevForks
	)
}
