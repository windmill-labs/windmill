import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/**
 * Which skills the chat may use, per workspace and per account.
 *
 * Being able to read a `skills` resource is not the same as wanting the chat to
 * follow it: a resource in a shared folder is readable by a whole team, and every
 * enabled skill puts its description in the model's context on every turn. So a
 * skill is off until it is turned on here.
 *
 * Stored per browser, like the chat's other per-user preferences, but keyed by
 * email as well as workspace: browser storage outlives a logout, and inheriting
 * the previous account's selection would hand the next person instructions they
 * never turned on.
 */
const KEY = 'wm_skills_enabled'

function scope(workspace: string): string | undefined {
	const email = get(userStore)?.email
	return email ? `${workspace}:${email}` : undefined
}

function read(): Record<string, string[]> {
	if (typeof localStorage === 'undefined') return {}
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '{}')
	} catch {
		return {}
	}
}

function write(all: Record<string, string[]>) {
	try {
		localStorage.setItem(KEY, JSON.stringify(all))
	} catch (e) {
		console.error('Failed to persist enabled skills', e)
	}
}

export function enabledSkillPaths(workspace: string): string[] {
	const key = scope(workspace)
	return key ? (read()[key] ?? []) : []
}

export function isSkillEnabled(workspace: string, path: string): boolean {
	return enabledSkillPaths(workspace).includes(path)
}

/** Returns false when there is no account to record the preference against, so a
 * caller that just created a skill can say it did not stay on. */
export function setSkillEnabled(workspace: string, path: string, enabled: boolean): boolean {
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
