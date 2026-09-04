import { List, Plug, Plus } from 'lucide-svelte'
import type { Component } from 'svelte'
import { get } from 'svelte/store'
import { ResourceService } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { sendUserToast } from '$lib/toast'
import type { Item } from '$lib/utils'
import type { AIChatManager } from '../copilot/chat/AIChatManager.svelte'
import { isMcpEnabled, setMcpEnabled } from './enabledServers'
import { cachedProviderKey, rememberProviderKey } from './iconCache'
import { loadProviderIcon } from './providerIcon'

type Row = {
	path: string
	editedAt?: string
	enabled: boolean
	icon?: Component<any>
}

// A menu is a shortcut, not a directory: past this many the list stops being
// scannable, so the rest are reached through the settings modal rather than dropped.
const MAX_MENU_SERVERS = 8
// A row whose provider is already cached paints from the cache; the rest cost one
// read each, and a long list stops asking rather than firing a request storm at a
// menu nobody is reading that far down.
const MAX_ICON_LOOKUPS = 20

/**
 * The chat "+" menu's MCP submenu: one row per connected server, checked when it
 * is on, then the way to manage them. Connecting and deleting live in the
 * assistant settings modal, which `onManage` opens.
 */
export class McpMenu {
	#manager: AIChatManager
	#onManage: () => void
	#seq = 0
	/** Rows for the workspace named by `#rowsWorkspace`, and meaningless for any other. */
	#rows = $state<Row[]>([])
	#rowsWorkspace: string | undefined = undefined

	constructor(manager: AIChatManager, onManage: () => void) {
		this.#manager = manager
		this.#onManage = onManage
	}

	// A session chat operates on its own (possibly forked) workspace without
	// switching `workspaceStore`, and that is the workspace the chat reads the
	// enabled set under. Key everything here the same way or a toggle lands under
	// a key nothing reads. Read per call rather than derived: the menu is built
	// on open, so there is no stale snapshot to keep current between opens.
	get #ws(): string | undefined {
		return this.#manager.operatingWorkspace ?? get(workspaceStore) ?? undefined
	}

	async #load(ws: string) {
		const seq = ++this.#seq
		try {
			const resources = await ResourceService.listResource({
				workspace: ws,
				resourceType: 'mcp',
				perPage: 100
			})
			if (seq !== this.#seq) return
			this.#rows = resources.map((r) => ({
				path: r.path,
				editedAt: r.edited_at,
				enabled: isMcpEnabled(ws, r.path)
			}))
			this.#rowsWorkspace = ws
			void this.#loadIcons(ws, seq)
		} catch {
			// The menu's other entries still work; an MCP submenu that failed to load
			// is better empty than blocking the whole "+" menu behind an error.
			if (seq !== this.#seq) return
			this.#rows = []
			this.#rowsWorkspace = ws
		}
	}

	async #loadIcons(ws: string, seq: number) {
		let lookups = 0
		await Promise.all(
			this.#rows.map(async (server) => {
				let key = cachedProviderKey(ws, server.path, server.editedAt)
				if (key === undefined) {
					if (lookups >= MAX_ICON_LOOKUPS) return
					lookups++
					try {
						const resource = await ResourceService.getResource({
							workspace: ws,
							path: server.path
						})
						key = rememberProviderKey(
							ws,
							server.path,
							(resource.value as { url?: unknown } | undefined)?.url,
							server.editedAt
						)
					} catch {
						return
					}
				}
				const icon = await loadProviderIcon(key)
				if (seq !== this.#seq) return
				server.icon = icon
			})
		)
	}

	#row(path: string) {
		return this.#rows.find((s) => s.path === path)
	}

	async #toggle(ws: string, path: string, enabled: boolean) {
		// A session whose fork is still staged has no workspace of its own yet, so `ws`
		// is the PARENT: the selection would be stored under it and quietly stop
		// applying the moment the first send commits the fork.
		const pendingForkOf = this.#manager.sessionContextResolver?.()?.pendingForkOf
		if (pendingForkOf !== undefined) {
			sendUserToast(
				`This session has not created its workspace yet, so the selection would be stored under "${pendingForkOf}". Send a message first.`,
				true
			)
			return
		}
		// Local preference only: nothing to re-read from the API, and the cached
		// tool lists stay valid because the servers are unchanged. Checked, like the
		// skills submenu: a refused write leaves the chat carrying a different set than
		// the check mark shows.
		if (!setMcpEnabled(ws, path, enabled)) {
			sendUserToast('Could not save the selection for this account.', true)
			return
		}
		const row = this.#row(path)
		if (row) row.enabled = enabled
		await this.#manager.refreshMcpServers(ws)
	}

	/** Loaded on open so the checks are current. */
	async items(closeMenu?: () => void): Promise<Item[]> {
		const ws = this.#ws
		if (!ws) return []
		// The menu opens on what is already known and refreshes behind it: awaited
		// inline it would stall the whole "+" menu, attachments included. Rows for
		// another workspace are not "already known" — same path, different server.
		if (this.#rowsWorkspace !== ws) {
			this.#rows = []
			await this.#load(ws)
		} else {
			void this.#load(ws)
		}
		// Enabled first: those are the ones a quick visit is most likely about.
		const ordered = [...this.#rows].sort(
			(a, b) => Number(b.enabled) - Number(a.enabled) || a.path.localeCompare(b.path)
		)
		const shown = ordered.slice(0, MAX_MENU_SERVERS)
		const manage = () => {
			closeMenu?.()
			this.#onManage()
		}
		// Bound out here because the getters below sit on plain object literals,
		// where `this` is the item rather than this menu.
		const row = (path: string) => this.#row(path)
		return [
			...shown.map(({ path }) => ({
				displayName: path,
				// Getters, not snapshots: the menu stays open across a click, and it has
				// to read through the live list rather than the row captured here, since
				// a reload replaces every row object and a getter bound to the old one
				// would go on reporting the state it was built with.
				get icon() {
					// Plug where the provider is unknown, so one nameless server does not
					// pull its label out of line with the rest.
					return row(path)?.icon ?? Plug
				},
				// Provider icons take css lengths and ignore lucide's `size`, so without
				// this one of them renders at its 24px default among 14px menu icons.
				get iconProps() {
					return row(path)?.icon ? { width: '14px', height: '14px' } : undefined
				},
				get toggle() {
					return row(path)?.enabled ?? false
				},
				action: () => this.#toggle(ws, path, !row(path)?.enabled)
			})),
			...(ordered.length > shown.length
				? [{ displayName: `Show all ${ordered.length}`, icon: List, action: manage }]
				: []),
			{
				displayName: 'Connect a server',
				icon: Plus,
				separatorTop: this.#rows.length > 0,
				action: manage
			}
		]
	}
}
