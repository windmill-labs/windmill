import { get, writable } from 'svelte/store'
import { WorkerService } from '$lib/gen'
import { userWorkspaces } from '$lib/stores'

/** Mirrors `DEFAULT_DEV_WORKSPACE_TAG_SUFFIX` in `windmill-common/src/worker.rs`. */
export const DEFAULT_DEV_WORKSPACE_TAG_SUFFIX = '-dev'
const WM_FORK_PREFIX = 'wm-fork-'

/** `null` while dev workspace isolation is off, `undefined` until loaded. */
export const devWorkspaceTagSuffix = writable<string | null | undefined>(undefined)

let inflight: Promise<string | null> | undefined
export function loadDevWorkspaceTagSuffix(force = false): Promise<string | null> {
	const current = get(devWorkspaceTagSuffix)
	if (!force && current !== undefined) return Promise.resolve(current)
	if (!force && inflight) return inflight
	// Started inside the chain so any failure lands in the catch: callers on a job's error path
	// await this and must not fail because the suffix could not be read.
	inflight = Promise.resolve()
		.then(() => WorkerService.getDevWorkspaceTagSuffix())
		.then((s) => {
			const suffix = s?.trim() ? s.trim() : null
			devWorkspaceTagSuffix.set(suffix)
			return suffix
		})
		.catch(() => {
			devWorkspaceTagSuffix.set(null)
			return null
		})
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

/** Whether a workspace's jobs run on dev workers: it has a parent, or is (or was) a fork. */
export function isDevOrForkWorkspace(w: {
	id: string
	parent_workspace_id?: string | null
}): boolean {
	return !!w.parent_workspace_id || w.id.startsWith(WM_FORK_PREFIX)
}

/** The same check for a workspace id, read from the user's workspaces. */
export function isDevOrForkWorkspaceId(workspaceId: string | undefined): boolean {
	if (!workspaceId) return false
	const w = get(userWorkspaces).find((w) => w.id === workspaceId)
	return isDevOrForkWorkspace(w ?? { id: workspaceId })
}
