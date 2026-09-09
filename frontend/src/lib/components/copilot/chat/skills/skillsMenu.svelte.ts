import { BookOpen, List, Plus } from 'lucide-svelte'
import { get } from 'svelte/store'
import { userStore, workspaceStore } from '$lib/stores'
import { sendUserToast } from '$lib/toast'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import type { Item } from '$lib/utils'
import type { AIChatManager } from '../AIChatManager.svelte'
import { isSkillEnabled, setSkillEnabled } from './enabledSkills'
import { ambiguousSkillNames, listSkillResources, type SkillResource } from './skillResources'

type Row = SkillResource & { enabled: boolean }

// A menu is a shortcut, not a directory: past this many the list stops being
// scannable, so the rest are reached through the settings modal rather than dropped.
const MAX_MENU_SKILLS = 8

/**
 * The chat "+" menu's Skills submenu: one row per skill, checked when it is on,
 * then the way to manage them. Everything a skill is beyond turning it on and off
 * lives in the assistant settings modal, which `onManage` opens.
 */
export class SkillsMenu {
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
			// The truncation flag is for the settings modal, which is where a partial
			// read is explained; the menu shows what it got.
			const { skills: found } = await listSkillResources(ws, get(userStore) ?? undefined)
			if (seq !== this.#seq) return
			this.#rows = found.map((s) => ({ ...s, enabled: isSkillEnabled(ws, s.path) }))
			this.#rowsWorkspace = ws
		} catch {
			// The menu's other entries still work; a skills submenu that failed to load
			// is better empty than blocking the whole "+" menu behind an error.
			if (seq !== this.#seq) return
			this.#rows = []
			this.#rowsWorkspace = ws
		}
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
		if (!setSkillEnabled(ws, path, enabled)) {
			sendUserToast('Could not save the selection for this account.', true)
			return
		}
		const row = this.#row(path)
		if (row) row.enabled = enabled
		// Whether people select skills at all. Never the skill itself: a path is
		// workspace-authored text.
		logFeatureUsage('ai_session', 'skill_toggle', { key: enabled ? 'on' : 'off', workspace: ws })
		// The prompt lists exactly the enabled skills, so it has to be rebuilt
		// before the next message rather than on the next mode change.
		await this.#manager.refreshGlobalSkills(ws)
	}

	/** Loaded on open so the checks are current. */
	async items(closeMenu?: () => void): Promise<Item[]> {
		const ws = this.#ws
		if (!ws) return []
		// The menu opens on what is already known and refreshes behind it: awaited
		// inline it would stall the whole "+" menu, attachments included. Rows for
		// another workspace are not "already known" — same path, different skill.
		if (this.#rowsWorkspace !== ws) {
			this.#rows = []
			await this.#load(ws)
		} else {
			void this.#load(ws)
		}
		const ambiguous = ambiguousSkillNames(this.#rows)
		// Enabled first: those are the ones a quick visit is most likely about.
		const ordered = [...this.#rows].sort(
			(a, b) => Number(b.enabled) - Number(a.enabled) || a.path.localeCompare(b.path)
		)
		const shown = ordered.slice(0, MAX_MENU_SKILLS)
		const manage = () => {
			closeMenu?.()
			this.#onManage()
		}
		// Bound out here because the getters below sit on plain object literals,
		// where `this` is the item rather than this menu.
		const row = (path: string) => this.#row(path)
		return [
			...shown.map(({ path, name }) => ({
				// Ambiguous names are shown by path — two rows reading `deploy` would
				// leave the choice between them to chance.
				displayName: ambiguous.has(name) ? path : name,
				icon: BookOpen,
				// Getters, not snapshots: the menu stays open across a click, and it has
				// to read through the live list rather than the row captured here, since
				// a reload replaces every row object and a getter bound to the old one
				// would go on reporting the state it was built with.
				get toggle() {
					return row(path)?.enabled ?? false
				},
				action: () => this.#toggle(ws, path, !row(path)?.enabled)
			})),
			...(ordered.length > shown.length
				? [{ displayName: `Show all ${ordered.length}`, icon: List, action: manage }]
				: []),
			{
				displayName: this.#rows.length > 0 ? 'Manage skills' : 'Add a skill',
				icon: Plus,
				separatorTop: this.#rows.length > 0,
				action: manage
			}
		]
	}
}
