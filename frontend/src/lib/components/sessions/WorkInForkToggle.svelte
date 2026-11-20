<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'

	const createWorkspaceModule = lazyModule(
		() => import('$lib/components/workspaceSettings/CreateWorkspaceInner.svelte')
	)

	// Drafts whose user unchecked the fork a protected workspace checked for them, by
	// `<session>:<workspace>`: it is not checked for them again until a reload.
	const declined = new Set<string>()
</script>

<script lang="ts">
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
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import { Button } from '$lib/components/common'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		type Session
	} from './sessionState.svelte'
	import { useSessionReadOnly } from './sessionPermissions.svelte'
	import { getUserExt } from '$lib/user'
	import {
		canUserBypassRuleKindInRulesets,
		isRuleActiveInRulesets,
		readProtectionRulesForWorkspace
	} from '$lib/workspaceProtectionRules.svelte'
	import {
		loadForkDefaults,
		saveForkDefaults,
		type StagedFork,
		type StagedForkSettings
	} from './sessionForkDefaults'

	let { session }: { session: Session } = $props()

	const WM_FORK_PREFIX = 'wm-fork-'
	// A fork is a new workspace, which community edition caps (backend _check_nb_of_workspaces).
	const CE_MAX_NON_ADMIN_WORKSPACES = 2

	// The header's workspace, which the draft acts on and a staged fork is staged from.
	const workspace = $derived(getEffectiveWorkspaceId(session))
	const root = $derived(findWorkspaceRoot(workspace, $userWorkspaces))
	const staged = $derived(session.pending_fork)

	// The workspace picker's own gate (WorkspaceFamilyPicker): forking is premium on cloud, and a
	// workspace that forbids forking can still be worked on through its dev workspace.
	const available = $derived(
		!!workspace &&
			workspace !== 'admins' &&
			!session.workspace_id &&
			(!isCloudHosted() || $maybePremium) &&
			(canCreateFork($userStore, $roleCapabilities) ||
				!!findCanonicalDevWorkspace(workspace, $userWorkspaces)) &&
			($enterpriseLicense ||
				$userWorkspaces.filter((w) => w.id !== 'admins').length < CE_MAX_NON_ADMIN_WORKSPACES)
	)

	function freshForkName(prefix: string): string {
		const taken = new Set($userWorkspaces.map((w) => w.id))
		for (let i = 0; i < 50; i++) {
			const candidate = random_adj()
			if (!taken.has(`${prefix}${candidate}`)) return candidate
		}
		const base = random_adj()
		let n = 1
		while (taken.has(`${prefix}${base}-${n}`)) n++
		return `${base}-${n}`
	}

	// A workspace has at most one dev workspace, so a saved dev choice applies only while the base
	// has none yet; otherwise the fork is a plain one.
	function applicableSettings(
		parent: string,
		settings: StagedForkSettings | undefined
	): StagedForkSettings | undefined {
		if (!settings?.is_dev_workspace) return settings
		const hasDev = $userWorkspaces.some(
			(w) => w.parent_workspace_id === parent && w.is_dev_workspace
		)
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

	// Whether the server lets this user fork `ws`: its `DisableWorkspaceForking` rule refuses
	// everyone it does not list as a bypass.
	async function forkingAllowed(ws: string): Promise<boolean> {
		const [rules, user] = await Promise.all([readProtectionRulesForWorkspace(ws), getUserExt(ws)])
		if (!rules) return false
		return (
			!isRuleActiveInRulesets(rules, 'DisableWorkspaceForking') ||
			canUserBypassRuleKindInRulesets(rules, 'DisableWorkspaceForking', user)
		)
	}

	// The family's saved base, while it is still a workspace of the family the user may fork, else
	// the default one. A base locked against forking gives way to its dev workspace.
	async function defaultBase(saved: string | undefined): Promise<string | undefined> {
		const family = root ? [root, ...findWorkspaceDescendants(root.id, $userWorkspaces)] : []
		const savedWs = family.find((w) => w.id === saved)
		const base =
			savedWs && canFork(capabilitiesIn(undefined, savedWs))
				? savedWs
				: findDefaultForkBase(workspace, $userWorkspaces)
		if (!base || (await forkingAllowed(base.id))) return base?.id
		return findCanonicalDevWorkspace(base.id, $userWorkspaces)?.id
	}

	let staging = false
	async function stageDefault(byUser: boolean) {
		if (staging) return
		staging = true
		const from = workspace
		const defaults = loadForkDefaults(root?.id)
		const parent = await defaultBase(defaults?.base).finally(() => (staging = false))
		// The header moved, or the draft was sent or staged, while the rules were read.
		if (!parent || workspace !== from || session.workspace_id || session.pending_fork) return
		const settings = applicableSettings(parent, defaults?.settings)
		// A dev workspace keeps its bare id.
		const prefix = settings?.is_dev_workspace ? '' : WM_FORK_PREFIX
		const name = freshForkName(prefix)
		setSessionPendingFork(
			session.id,
			{ parent_workspace_id: parent, id: `${prefix}${name}`, name, settings },
			{ byUser }
		)
	}

	function toggle() {
		if (staged) {
			if (protectedWorkspace.known) declined.add(`${session.id}:${workspace}`)
			setSessionPendingFork(session.id, undefined)
		} else {
			void stageDefault(true)
		}
	}

	// A protected workspace holds the session in Use, so its draft starts in a fork.
	const protectedWorkspace = useSessionReadOnly(() => workspace)
	$effect(() => {
		if (
			available &&
			protectedWorkspace.known &&
			!staged &&
			!declined.has(`${session.id}:${workspace}`)
		)
			void stageDefault(false)
	})

	let settingsOpen = $state(false)
	function openSettings() {
		void createWorkspaceModule.load()
		settingsOpen = true
	}

	function saveSettings(fork: StagedFork) {
		setSessionPendingFork(session.id, fork)
		// Everything but the name, which each fork gets fresh.
		saveForkDefaults(root?.id, { base: fork.parent_workspace_id, settings: fork.settings ?? {} })
	}
</script>

{#if available}
	<div class="flex items-center gap-1.5 min-w-0 text-2xs text-secondary">
		<label class="flex items-center gap-1.5 shrink-0 cursor-pointer">
			<Checkbox checked={!!staged} onChange={toggle} />
			<span>Work in fork</span>
		</label>
		{#if staged}
			<Button
				variant="subtle"
				unifiedSize="2xs"
				btnClasses="min-w-0 !text-2xs font-medium"
				title="Edit the settings this fork is created with when you send your first message"
				onClick={openSettings}
			>
				<span class="truncate">{staged.name}</span>
			</Button>
		{/if}
	</div>
{/if}

<Modal2
	title="Forking {workspace}"
	target="#content"
	fixedHeight="adaptive"
	fixedWidth="sm"
	contentClasses="flex-col"
	bind:isOpen={settingsOpen}
>
	{#if settingsOpen && staged && createWorkspaceModule.current}
		{@const CreateWorkspaceInner = createWorkspaceModule.current.default}
		<CreateWorkspaceInner
			isFork
			inModal
			stage={{ fork: staged, onStage: saveSettings }}
			onFinish={() => (settingsOpen = false)}
		/>
	{/if}
</Modal2>
