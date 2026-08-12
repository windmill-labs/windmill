import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/**
 * Which MCP servers the chat may use, per workspace and per account.
 *
 * Being able to read an `mcp` resource is not the same as wanting the chat to
 * act through it: a resource in a shared folder is readable by a whole team, and
 * each server's tools both reach an external system and put their descriptions
 * in the model's context. So a server is off until it is turned on here, and
 * connecting one through the chat turns it on for the person who connected it.
 *
 * Stored per browser, like the chat's other per-user preferences, but keyed by
 * email as well as workspace: browser storage outlives a logout, and inheriting
 * the previous account's enabled servers would hand the next person tools they
 * never turned on.
 */
const KEY = 'wm_mcp_enabled'

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
		console.error('Failed to persist enabled MCP servers', e)
	}
}

export function enabledMcpPaths(workspace: string): string[] {
	const key = scope(workspace)
	return key ? (read()[key] ?? []) : []
}

export function isMcpEnabled(workspace: string, path: string): boolean {
	return enabledMcpPaths(workspace).includes(path)
}

export function setMcpEnabled(workspace: string, path: string, enabled: boolean) {
	const key = scope(workspace)
	if (!key) return
	const all = read()
	const current = new Set(all[key] ?? [])
	if (enabled) {
		current.add(path)
	} else {
		current.delete(path)
	}
	all[key] = [...current]
	write(all)
}
