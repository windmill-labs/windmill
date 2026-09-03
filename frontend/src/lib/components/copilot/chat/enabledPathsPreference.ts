import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/**
 * A set of workspace-object paths the chat may act through, remembered per
 * workspace and per account.
 *
 * Being able to read a resource is not the same as wanting the chat to use it: a
 * resource in a shared folder is readable by a whole team, and each enabled entry
 * costs something on every turn — an MCP server puts its tool descriptions in the
 * model's context and reaches an external system, a skill puts its description
 * there. So an entry is off until it is turned on.
 *
 * Stored per browser, like the chat's other per-user preferences, but keyed by
 * email as well as workspace: browser storage outlives a logout, and inheriting
 * the previous account's selection would hand the next person capabilities they
 * never turned on. Workspace ids cannot contain `:`, so the composite key is
 * unambiguous.
 */
export type EnabledPathsPreference = {
	enabledPaths: (workspace: string) => string[]
	isEnabled: (workspace: string, path: string) => boolean
	/** Returns false when there is no account to record the preference against, so
	 * a caller that just created the object can say it did not stay on. */
	setEnabled: (workspace: string, path: string, enabled: boolean) => boolean
}

export function createEnabledPathsPreference(storageKey: string): EnabledPathsPreference {
	function scope(workspace: string): string | undefined {
		const email = get(userStore)?.email
		return email ? `${workspace}:${email}` : undefined
	}

	function read(): Record<string, string[]> {
		if (typeof localStorage === 'undefined') return {}
		try {
			return JSON.parse(localStorage.getItem(storageKey) ?? '{}')
		} catch {
			return {}
		}
	}

	function write(all: Record<string, string[]>) {
		try {
			localStorage.setItem(storageKey, JSON.stringify(all))
		} catch (e) {
			console.error(`Failed to persist ${storageKey}`, e)
		}
	}

	function enabledPaths(workspace: string): string[] {
		const key = scope(workspace)
		return key ? (read()[key] ?? []) : []
	}

	return {
		enabledPaths,
		isEnabled: (workspace, path) => enabledPaths(workspace).includes(path),
		setEnabled: (workspace, path, enabled) => {
			const key = scope(workspace)
			if (!key) return false
			const all = read()
			const current = new Set(all[key] ?? [])
			if (enabled) {
				current.add(path)
			} else {
				current.delete(path)
			}
			all[key] = [...current]
			write(all)
			return true
		}
	}
}
