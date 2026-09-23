import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/**
 * Which workspace-object paths the chat may act through, remembered per workspace
 * and per account.
 *
 * Only the paths someone actually decided about are stored; everything else is the
 * kind's default. That is what lets a default flip — skills went from off-until-on
 * to on-until-off — without rewriting anyone's storage: the stored entries keep
 * meaning what the person chose, and only the paths they never touched move.
 *
 * Stored per browser, like the chat's other per-user preferences, but keyed by
 * email as well as workspace: browser storage outlives a logout, and one person's
 * decisions must not be read as the next person's — whichever way they went.
 * Workspace ids cannot contain `:`, so the composite key is unambiguous.
 */
export type PathsPreference = {
	/** Paths stored as on, which is the enabled set only for a kind that defaults to
	 * off. For one that defaults to on this is whatever happens to be written down —
	 * nothing for a decision made since the default flipped, and the whole selection
	 * for a browser still carrying storage from before it. Neither is the answer to
	 * "what is enabled": ask `isEnabled` per path for that. */
	explicitlyEnabledPaths: (workspace: string) => string[]
	isEnabled: (workspace: string, path: string) => boolean
	/** Returns false when there is no account to record the preference against, so
	 * a caller that just created the object can say it did not stay on. */
	setEnabled: (workspace: string, path: string, enabled: boolean) => boolean
	/** Drop any decision about `path`, leaving it at the default. What a caller wants
	 * when the thing at that path is gone — a deleted or renamed skill — rather than
	 * writing the default as a decision, which reads as one and would mean the
	 * opposite the day the default moves. */
	forget: (workspace: string, path: string) => void
}

/** One decision per path. The older shape was an array of the paths that were on,
 * which says exactly that and so needs no conversion to be read. */
type Choices = Record<string, boolean>

function toChoices(raw: unknown): Choices {
	if (Array.isArray(raw)) return Object.fromEntries(raw.map((path) => [String(path), true]))
	if (raw && typeof raw === 'object') {
		return Object.fromEntries(
			Object.entries(raw as Record<string, unknown>).map(([path, on]) => [path, on === true])
		)
	}
	return {}
}

export function createPathsPreference(
	storageKey: string,
	defaultEnabled: boolean
): PathsPreference {
	function scope(workspace: string): string | undefined {
		const email = get(userStore)?.email
		return email ? `${workspace}:${email}` : undefined
	}

	function read(): Record<string, unknown> {
		if (typeof localStorage === 'undefined') return {}
		try {
			return JSON.parse(localStorage.getItem(storageKey) ?? '{}')
		} catch {
			return {}
		}
	}

	function choices(workspace: string): Choices {
		const key = scope(workspace)
		return key ? toChoices(read()[key]) : {}
	}

	/** `decision` of undefined drops the entry. */
	function write(workspace: string, path: string, decision: boolean | undefined): boolean {
		const key = scope(workspace)
		if (!key) return false
		const all = read()
		const current = toChoices(all[key])
		if (decision === undefined) {
			delete current[path]
		} else {
			current[path] = decision
		}
		all[key] = current
		try {
			localStorage.setItem(storageKey, JSON.stringify(all))
		} catch (e) {
			console.error(`Failed to persist ${storageKey}`, e)
		}
		return true
	}

	return {
		explicitlyEnabledPaths: (workspace) =>
			Object.entries(choices(workspace))
				.filter(([, on]) => on)
				.map(([path]) => path),
		isEnabled: (workspace, path) => choices(workspace)[path] ?? defaultEnabled,
		// A path put back to the default is dropped rather than stored as one: the
		// entries are the decisions, so a re-enabled skill leaves nothing behind and a
		// later skill at that path starts from the default like any other.
		setEnabled: (workspace, path, enabled) =>
			write(workspace, path, enabled === defaultEnabled ? undefined : enabled),
		forget: (workspace, path) => void write(workspace, path, undefined)
	}
}
