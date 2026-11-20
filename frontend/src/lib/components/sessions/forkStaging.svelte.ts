import { fromStore } from 'svelte/store'
import {
	enterpriseLicense,
	maybePremium,
	roleCapabilities,
	userStore,
	userWorkspaces
} from '$lib/stores'
import {
	findCanonicalDevWorkspace,
	findDefaultForkBase,
	findWorkspaceDescendants,
	findWorkspaceRoot
} from '$lib/utils/workspaceHierarchy'
import { canCreateFork } from '$lib/utils/editInFork'
import { canFork, capabilitiesIn } from '$lib/roleCapabilities'
import { isCloudHosted } from '$lib/cloud'
import { random_adj } from '$lib/components/random_positive_adjetive'
import { devBadgeText } from '$lib/utils/devWorkspaceLabel'
import { getUserExt } from '$lib/user'
import { WorkspaceService } from '$lib/gen'
import {
	canUserBypassRuleKindInRulesets,
	isRuleActiveInRulesets,
	readProtectionRulesForWorkspace
} from '$lib/workspaceProtectionRules.svelte'
import { getEffectiveWorkspaceId, setSessionPendingFork, type Session } from './sessionState.svelte'
import {
	loadForkDefaults,
	saveForkDefaults,
	type StagedFork,
	type StagedForkSettings
} from './sessionForkDefaults'

const WM_FORK_PREFIX = 'wm-fork-'
// A fork is a new workspace, which community edition caps (backend _check_nb_of_workspaces).
const CE_MAX_NON_ADMIN_WORKSPACES = 2

/**
 * Staging a new fork for a Home draft's first send: whether the user may, and the fork staged
 * with the family's saved defaults under a fresh name. Reads stores: call during component
 * initialisation.
 */
export function useForkStaging(session: () => Session | undefined) {
	const workspaces = fromStore(userWorkspaces)
	const user = fromStore(userStore)
	const caps = fromStore(roleCapabilities)
	const premium = fromStore(maybePremium)
	const license = fromStore(enterpriseLicense)

	// The header's workspace, which the draft acts on and a staged fork is staged from.
	const workspace = $derived.by(() => {
		const s = session()
		return s ? getEffectiveWorkspaceId(s) : undefined
	})
	const root = $derived(findWorkspaceRoot(workspace, workspaces.current))

	// The workspace picker's own gate (WorkspaceFamilyPicker): forking is premium on cloud, and a
	// workspace that forbids forking can still be worked on through its dev workspace.
	const available = $derived(
		!!workspace &&
			workspace !== 'admins' &&
			!session()?.workspace_id &&
			(!isCloudHosted() || !!premium.current) &&
			(canCreateFork(user.current, caps.current) ||
				!!findCanonicalDevWorkspace(workspace, workspaces.current)) &&
			(!!license.current ||
				workspaces.current.filter((w) => w.id !== 'admins').length < CE_MAX_NON_ADMIN_WORKSPACES)
	)

	// Workspace ids are unique across the instance, and the list holds only the user's own: the
	// server has the last word on whether an id is free.
	async function isTaken(id: string): Promise<boolean> {
		if (workspaces.current.some((w) => w.id === id)) return true
		return await WorkspaceService.existsWorkspace({ requestBody: { id } }).catch(() => false)
	}

	async function freeId(base: string): Promise<string> {
		if (!(await isTaken(base))) return base
		let n = 2
		while (await isTaken(`${base}-${n}`)) n++
		return `${base}-${n}`
	}

	async function freshForkName(prefix: string): Promise<string> {
		for (let i = 0; i < 20; i++) {
			const candidate = random_adj()
			if (!(await isTaken(`${prefix}${candidate}`))) return candidate
		}
		const base = random_adj()
		let n = 1
		while (await isTaken(`${prefix}${base}-${n}`)) n++
		return `${base}-${n}`
	}

	// A workspace has at most one dev workspace, so a saved dev choice applies only while the base
	// has none yet, and never on a dev base, which the fork form offers no dev of from a session;
	// otherwise the fork is a plain one. The server is asked, as the user may not see that dev.
	async function applicableSettings(
		parent: string,
		settings: StagedForkSettings | undefined
	): Promise<StagedForkSettings | undefined> {
		if (!settings?.is_dev_workspace) return settings
		const parentIsDev = workspaces.current.some((w) => w.id === parent && w.is_dev_workspace)
		const hasDev =
			parentIsDev ||
			(await WorkspaceService.getDevWorkspace({ workspace: parent }).then(
				(d) => !!d,
				() => true
			))
		if (!hasDev) return settings
		return {
			...settings,
			is_dev_workspace: false,
			dev_workspace_label: undefined,
			lock_prod_deploy: undefined,
			lock_prod_forking: undefined,
			copy_members: false
		}
	}

	// How `ws`'s `DisableWorkspaceForking` rule treats this user: it refuses everyone it does not
	// list as a bypass, and a bypass is an explicit choice in the fork form, never a default.
	async function forkingRule(ws: string): Promise<'free' | 'bypass' | 'blocked'> {
		const [rules, me] = await Promise.all([readProtectionRulesForWorkspace(ws), getUserExt(ws)])
		if (!rules) return 'blocked'
		if (!isRuleActiveInRulesets(rules, 'DisableWorkspaceForking')) return 'free'
		return canUserBypassRuleKindInRulesets(rules, 'DisableWorkspaceForking', me)
			? 'bypass'
			: 'blocked'
	}

	// The family's saved base, while it is still a workspace of the family the user may fork, else
	// the default one. A base locked against forking gives way to its dev workspace; a bypasser
	// without one keeps the base, and the form asks them to confirm the bypass.
	async function defaultBase(saved: string | undefined): Promise<string | undefined> {
		const family = root ? [root, ...findWorkspaceDescendants(root.id, workspaces.current)] : []
		const savedWs = family.find((w) => w.id === saved)
		const base =
			savedWs && canFork(capabilitiesIn(undefined, savedWs))
				? savedWs
				: findDefaultForkBase(workspace, workspaces.current)
		if (!base) return undefined
		const rule = await forkingRule(base.id)
		if (rule === 'free') return base.id
		const dev = findCanonicalDevWorkspace(base.id, workspaces.current)?.id
		return dev ?? (rule === 'bypass' ? base.id : undefined)
	}

	let staging = false
	/** Stage a fork with the saved defaults; whether one is staged once it returns. */
	async function stage(): Promise<boolean> {
		const s = session()
		if (!s || staging) return !!s?.pending_fork
		staging = true
		const from = workspace
		const defaults = loadForkDefaults(root?.id)
		try {
			const parent = await defaultBase(defaults?.base)
			if (!parent) return !!session()?.pending_fork
			const settings = await applicableSettings(parent, defaults?.settings)
			// A dev workspace is named `<base>-<label>` with no `wm-fork-` prefix, as the fork form names it.
			const prefix = settings?.is_dev_workspace ? '' : WM_FORK_PREFIX
			const name = settings?.is_dev_workspace
				? await freeId(`${parent}-${devBadgeText(settings.dev_workspace_label)}`)
				: await freshForkName(prefix)
			const now = session()
			// The header moved, or the draft was sent or staged, while the server was asked.
			if (!now || workspace !== from || now.workspace_id) return !!now?.pending_fork
			if (now.pending_fork) return true
			setSessionPendingFork(now.id, {
				parent_workspace_id: parent,
				id: `${prefix}${name}`,
				name,
				settings
			})
			return true
		} finally {
			staging = false
		}
	}

	return {
		get available() {
			return available
		},
		get workspace() {
			return workspace
		},
		stage,
		unstage() {
			const s = session()
			if (s) setSessionPendingFork(s.id, undefined)
		},
		/** Keep `fork` for this draft, and all of it but its name as the family's defaults. */
		save(fork: StagedFork) {
			const s = session()
			if (!s) return
			setSessionPendingFork(s.id, fork)
			saveForkDefaults(root?.id, { base: fork.parent_workspace_id, settings: fork.settings ?? {} })
		}
	}
}
