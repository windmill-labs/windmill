<script lang="ts">
	import { tick, type Snippet } from 'svelte'
	import {
		enterpriseLicense,
		isPremiumStore,
		userStore,
		userWorkspaces,
		workspaceStore,
		type UserWorkspace
	} from '$lib/stores'
	import {
		findWorkspaceDescendants,
		findCanonicalDevWorkspace
	} from '$lib/utils/workspaceHierarchy'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { getUserExt } from '$lib/user'
	import {
		fetchProtectionRulesForWorkspace,
		isRuleActiveInRulesets,
		canUserBypassRuleKindInRulesets
	} from '$lib/workspaceProtectionRules.svelte'
	import { resource } from 'runed'
	import { isCloudHosted } from '$lib/cloud'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import InputError from '$lib/components/InputError.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Badge } from '$lib/components/common'
	import { Building, Check, GitFork, Plus } from 'lucide-svelte'

	type PendingFork = { parent_workspace_id: string; id: string; name: string }
	type ForkRequest = { parent_workspace_id: string; id: string; name: string }

	let {
		// Workspace currently associated with the consumer (for display only —
		// drives the family-root resolution and the "selected" highlight).
		// Defaults to the active workspace store when not set.
		selectedId,
		// A staged-but-not-yet-created fork (e.g. SessionWorkspaceBar's
		// pre-send draft). Highlighted as the active row when set.
		pendingFork,
		onPick,
		onCreateFork,
		allowCreateFork = true,
		// Optional caption rendered under the new-fork input. Lets the
		// consumer differentiate "staged for first send" vs. "will be
		// created immediately" semantics.
		createForkCaption = '',
		trigger
	}: {
		selectedId?: string
		pendingFork?: PendingFork
		onPick: (workspaceId: string) => void | Promise<void>
		onCreateFork?: (fork: ForkRequest) => void | Promise<void>
		allowCreateFork?: boolean
		createForkCaption?: string
		trigger: Snippet<[{ open: boolean }]>
	} = $props()

	const WM_FORK_PREFIX = 'wm-fork-'

	function findRoot(id: string | undefined, all: UserWorkspace[]): UserWorkspace | undefined {
		if (!id) return undefined
		let current = all.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = all.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	const effectiveId = $derived(selectedId ?? $workspaceStore ?? undefined)
	const root = $derived(findRoot(effectiveId, $userWorkspaces))
	const forks = $derived(root ? findWorkspaceDescendants(root.id, $userWorkspaces) : [])

	// New forks derive from the editable dev workspace when the family has one:
	// the root (prod) is typically forking-locked, so a non-admin can't fork it,
	// and dev holds the current code anyway. Falls back to the root otherwise.
	const devOfRoot = $derived(root ? findCanonicalDevWorkspace(root.id, $userWorkspaces) : undefined)
	const forkSource = $derived(devOfRoot ?? root)
	const createForkLabel = $derived(devOfRoot ? `Fork from ${devOfRoot.name}` : 'Create new fork…')

	// Judge the prod root off its OWN protection rules (fetched), not the active
	// workspace's — so it reads correctly from a dev/fork too, the same way
	// ParentWorkspaceProtectionAlert checks the parent. It's selectable when you
	// can deploy to it, and "Fork from prod" shows when you can fork it.
	const rootRulesetsResource = resource(
		() => root?.id,
		async (id) => (id ? await fetchProtectionRulesForWorkspace(id) : [])
	)
	const rootRulesets = $derived(rootRulesetsResource.current ?? [])
	// Bypass must be judged with the user's identity IN THE ROOT (is_admin/groups are per-workspace),
	// not the active workspace's `$userStore` — otherwise an admin of the dev/active workspace who is a
	// non-admin of the root would get a false bypass. `getUserExt` returns undefined for a non-member,
	// which `canUserBypassRuleKindInRulesets` treats as no bypass.
	const rootUserInfoResource = resource(
		() => root?.id,
		async (id) => (id ? await getUserExt(id) : undefined)
	)
	const rootUserInfo = $derived(rootUserInfoResource.current)
	const canDeployRoot = $derived(
		!isRuleActiveInRulesets(rootRulesets, 'DisableDirectDeployment') ||
			canUserBypassRuleKindInRulesets(rootRulesets, 'DisableDirectDeployment', rootUserInfo)
	)
	const canForkRoot = $derived(
		!isRuleActiveInRulesets(rootRulesets, 'DisableWorkspaceForking') ||
			canUserBypassRuleKindInRulesets(rootRulesets, 'DisableWorkspaceForking', rootUserInfo)
	)
	// A genuinely deploy-locked root (you can't deploy and can't bypass) is disabled regardless of
	// whether a dev workspace exists to steer to — being deploy-locked is the gate, not dev presence.
	// Roots with no rules resolve `canDeployRoot` to true, so ordinary families aren't affected. Kept
	// disabled while either fetch is in flight (both default to the conservative locked state).
	const rootDisabled = $derived(
		rootRulesetsResource.loading || rootUserInfoResource.loading || !canDeployRoot
	)

	// Structural gate: hidden in the admins workspace, or when the user can't fork; on cloud, forking
	// is premium-only (backend caps it per paid seat). DisableWorkspaceForking on the active workspace
	// (a locked prod) doesn't apply when there's a dev to fork from instead — the dev isn't locked, and
	// devOfRoot only resolves when the user is a member of it.
	const forksGateOpen = $derived(
		(!isCloudHosted() || $isPremiumStore) &&
			$workspaceStore !== 'admins' &&
			(canCreateFork($userStore) || !!devOfRoot)
	)
	// A fork is a new workspace, so it's subject to the community-edition cap on
	// the number of non-'admins' workspaces (backend _check_nb_of_workspaces,
	// run only on community builds). An enterprise license lifts the cap. We
	// mirror the backend count with the client-side workspace list to hide the
	// affordance once the cap is reached; the server still enforces the real
	// (instance-wide) check on commit, so this is purely UX.
	const CE_MAX_NON_ADMIN_WORKSPACES = 2
	const nonAdminWorkspaceCount = $derived($userWorkspaces.filter((w) => w.id !== 'admins').length)
	const ceWorkspaceCapReached = $derived(
		!$enterpriseLicense && nonAdminWorkspaceCount >= CE_MAX_NON_ADMIN_WORKSPACES
	)
	// The interactive create-fork row is shown unless the cap is reached;
	// otherwise (structural gate open but cap hit) we surface a disabled row
	// explaining the limit — never stage a fork the backend would reject.
	const forkAffordanceOpen = $derived(allowCreateFork && forksGateOpen && !!onCreateFork && !!root)
	const showCreateFork = $derived(forkAffordanceOpen && !ceWorkspaceCapReached)
	const showForkUpsell = $derived(forkAffordanceOpen && ceWorkspaceCapReached)
	// Whether the second "Fork from <root>" create entry is shown (admins can fork the prod root
	// directly even when the default entry forks the dev). Mirrors its render condition so keyboard
	// nav and the row index math stay in sync. Suppressed while the root's rules are still loading,
	// otherwise `canForkRoot` defaults true and a non-bypass user could stage a fork from a
	// forking-locked root before the rules resolve.
	const hasCreateFromRoot = $derived(
		showCreateFork &&
			!rootRulesetsResource.loading &&
			!rootUserInfoResource.loading &&
			canForkRoot &&
			!!devOfRoot
	)

	let dropdownOpen = $state(false)
	let creatingFork = $state(false)
	let newForkName = $state('')
	let forkInput: TextInput | undefined = $state(undefined)
	// Admins get a second entry to fork the prod root directly (the default forks
	// the dev). `forkFromRoot` tracks which source the create input is for.
	let forkFromRoot = $state(false)
	const effectiveForkSource = $derived(forkFromRoot ? root : forkSource)

	// Manual keyboard navigation, modelled after SelectDropdown. melt's
	// menu API couples Enter/Space to closing the menu, which we explicitly
	// don't want for the "Create new fork" row — it swaps to inline input.
	type NavRow =
		| { kind: 'create' }
		| { kind: 'create-from-root' }
		| { kind: 'root'; id: string }
		| { kind: 'fork'; id: string }
	const navRows = $derived<NavRow[]>([
		...(showCreateFork ? [{ kind: 'create' as const }] : []),
		...(hasCreateFromRoot ? [{ kind: 'create-from-root' as const }] : []),
		...(root ? [{ kind: 'root' as const, id: root.id }] : []),
		...forks.map((f) => ({ kind: 'fork' as const, id: f.id }))
	])
	let keyArrowPos = $state<number | undefined>(undefined)
	$effect(() => {
		if (!dropdownOpen) keyArrowPos = undefined
	})

	function activateRow(row: NavRow) {
		if (row.kind === 'create') {
			void enterCreateMode()
		} else if (row.kind === 'create-from-root') {
			void enterCreateMode(undefined, true)
		} else if (row.kind === 'root') {
			if (!rootDisabled) void pick(row.id)
		} else if (row.kind === 'fork') {
			void pick(row.id)
		}
	}

	function defaultForkName(): string {
		const taken = new Set($userWorkspaces.map((w) => w.id))
		if (pendingFork) taken.add(pendingFork.id)
		for (let i = 0; i < 50; i++) {
			const name = `${random_adj()}-fork`
			if (!taken.has(`${WM_FORK_PREFIX}${name}`)) return name
		}
		const base = `${random_adj()}-fork`
		let n = 1
		while (taken.has(`${WM_FORK_PREFIX}${base}-${n}`)) n++
		return `${base}-${n}`
	}

	async function pick(id: string) {
		dropdownOpen = false
		creatingFork = false
		await onPick(id)
	}

	async function enterCreateMode(initialName?: string, fromRoot = false) {
		forkFromRoot = fromRoot
		creatingFork = true
		newForkName = initialName ?? defaultForkName()
		await tick()
		forkInput?.focus()
		forkInput?.select()
	}

	function cancelCreate() {
		creatingFork = false
		newForkName = ''
		forkFromRoot = false
	}

	function slugForkBaseId(name: string): string {
		return name
			.toLowerCase()
			.replace(/[^a-z0-9-]/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '')
	}

	const forkNameError = $derived.by<string | undefined>(() => {
		const trimmed = newForkName.trim()
		if (!trimmed) return undefined
		const baseId = slugForkBaseId(trimmed)
		if (!baseId) return 'Name must contain at least one letter or number'
		const prefixed = `${WM_FORK_PREFIX}${baseId}`
		const taken = new Set($userWorkspaces.map((w) => w.id))
		if (pendingFork) taken.delete(pendingFork.id)
		if (taken.has(prefixed)) return 'A workspace with this name already exists'
		return undefined
	})

	async function stageNewFork() {
		const name = newForkName.trim()
		if (!effectiveForkSource || !name || forkNameError || !onCreateFork) return
		const baseId = slugForkBaseId(name)
		if (!baseId) return
		const prefixed = `${WM_FORK_PREFIX}${baseId}`
		// Close optimistically; consumer can re-open + toast on error.
		creatingFork = false
		newForkName = ''
		forkFromRoot = false
		dropdownOpen = false
		await onCreateFork({ parent_workspace_id: effectiveForkSource.id, id: prefixed, name })
	}

	function isSelected(id: string): boolean {
		if (pendingFork?.id === id) return true
		return !pendingFork && effectiveId === id
	}

	// Reopening the dropdown while a pending fork is staged drops the user
	// directly into edit mode so they can refine the name. Avoids re-
	// entering edit mode after an explicit cancel.
	let lastDropdownOpen = $state(false)
	$effect(() => {
		const wasOpen = lastDropdownOpen
		lastDropdownOpen = dropdownOpen
		if (dropdownOpen && !wasOpen && pendingFork && !creatingFork && showCreateFork) {
			// Preserve which source the fork was staged from (root vs dev); without this the re-entry
			// defaults to the dev and silently re-parents a "Fork from <root>" request.
			void enterCreateMode(pendingFork.name, pendingFork.parent_workspace_id === root?.id)
		}
	})
</script>

<svelte:window
	onkeydown={(e) => {
		if (!dropdownOpen) return
		if (creatingFork) return
		if (navRows.length === 0) return
		if (e.key === 'ArrowDown') {
			keyArrowPos = keyArrowPos === undefined ? 0 : Math.min(navRows.length - 1, keyArrowPos + 1)
			e.preventDefault()
		} else if (e.key === 'ArrowUp') {
			keyArrowPos = keyArrowPos === undefined ? navRows.length - 1 : Math.max(0, keyArrowPos - 1)
			e.preventDefault()
		} else if (e.key === 'Enter' && keyArrowPos !== undefined) {
			activateRow(navRows[keyArrowPos])
			e.preventDefault()
		} else if (e.key === 'Escape') {
			dropdownOpen = false
			e.preventDefault()
		}
	}}
/>

<DropdownV2
	bind:open={dropdownOpen}
	customMenu
	placement="bottom-start"
	fixedHeight={false}
	usePointerDownOutside
>
	{#snippet buttonReplacement()}
		{@render trigger({ open: dropdownOpen })}
	{/snippet}
	{#snippet menu()}
		{@const rowBase =
			'px-3 py-1.5 text-xs text-primary flex flex-row gap-2 items-center text-left rounded-sm w-full'}
		<div
			class="bg-surface-tertiary dark:border w-64 origin-top-left rounded-lg shadow-lg focus:outline-none py-1 flex flex-col max-h-80 overflow-y-auto"
		>
			{#if showCreateFork}
				{#if creatingFork}
					<div class="flex flex-col gap-1 px-2 py-1.5">
						<div class="flex flex-row items-center gap-1.5">
							<Plus size={14} class="shrink-0 text-tertiary" />
							<!-- svelte-ignore a11y_autofocus -->
							<TextInput
								bind:this={forkInput}
								bind:value={newForkName}
								size="xs"
								error={forkNameError}
								class="flex-1 min-w-0"
								inputProps={{
									placeholder: 'Fork name',
									autofocus: true,
									'aria-invalid': forkNameError ? 'true' : undefined,
									onkeydown: (e: KeyboardEvent) => {
										if (e.key === 'Enter') {
											e.preventDefault()
											e.stopPropagation()
											void stageNewFork()
										} else if (e.key === 'Escape') {
											e.preventDefault()
											e.stopPropagation()
											cancelCreate()
										}
									}
								}}
							/>
							<button
								type="button"
								aria-label="Confirm"
								title="Stage fork"
								onclick={() => void stageNewFork()}
								disabled={!newForkName.trim() || !!forkNameError}
								class="inline-flex items-center justify-center w-5 h-5 rounded text-accent hover:bg-surface-hover disabled:opacity-40 disabled:cursor-not-allowed"
							>
								<Check size={14} />
							</button>
						</div>
						{#if forkNameError || createForkCaption}
							<div class="pl-6">
								<InputError error={forkNameError} />
								{#if !forkNameError && createForkCaption}
									<span class="text-2xs text-tertiary">{createForkCaption}</span>
								{/if}
							</div>
						{/if}
					</div>
				{:else}
					{@const createIdx = 0}
					<button
						type="button"
						class={`${rowBase} ${keyArrowPos === createIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
						onmouseenter={() => (keyArrowPos = createIdx)}
						onclick={() => enterCreateMode()}
					>
						<Plus size={14} class="shrink-0 text-tertiary" />
						<span>{createForkLabel}</span>
					</button>
					{#if hasCreateFromRoot}
						{@const createFromRootIdx = 1}
						<button
							type="button"
							class={`${rowBase} ${keyArrowPos === createFromRootIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
							onmouseenter={() => (keyArrowPos = createFromRootIdx)}
							onclick={() => enterCreateMode(undefined, true)}
						>
							<Plus size={14} class="shrink-0 text-tertiary" />
							<span>Fork from {root?.name}</span>
						</button>
					{/if}
				{/if}
				<div class="my-1 border-t border-border-light shrink-0"></div>
			{:else if showForkUpsell}
				<div
					class={`${rowBase} opacity-60 cursor-not-allowed`}
					aria-disabled="true"
					title="Community edition is limited to {CE_MAX_NON_ADMIN_WORKSPACES +
						1} workspaces. Archive a workspace or upgrade to an enterprise license to create more forks."
				>
					<Plus size={14} class="shrink-0 text-tertiary" />
					<span>{createForkLabel}</span>
					<span class="ml-auto shrink-0 text-2xs text-tertiary"> Workspace limit reached </span>
				</div>
				<div class="my-1 border-t border-border-light shrink-0"></div>
			{/if}

			{#if root}
				{@const rootIdx = (showCreateFork ? 1 : 0) + (hasCreateFromRoot ? 1 : 0)}
				<button
					type="button"
					disabled={rootDisabled}
					title={rootDisabled
						? devOfRoot
							? `${root.name} is locked. Run in its dev workspace instead.`
							: `${root.name} is locked for direct deploys.`
						: undefined}
					class={`${rowBase} ${rootDisabled ? 'opacity-50 cursor-not-allowed' : ''} ${isSelected(root.id) && !pendingFork ? 'bg-surface-selected' : ''} ${!rootDisabled && keyArrowPos === rootIdx ? 'bg-surface-hover' : !rootDisabled ? 'hover:bg-surface-hover' : ''}`}
					onmouseenter={() => !rootDisabled && (keyArrowPos = rootIdx)}
					onclick={() => !rootDisabled && void pick(root.id)}
				>
					<Building size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{root.name}</span>
					<span class="text-2xs text-tertiary shrink-0 ml-auto"
						>{rootDisabled ? 'locked' : 'root'}</span
					>
				</button>
			{/if}
			{#each forks as f, fi (f.id)}
				{@const forkIdx =
					(showCreateFork ? 1 : 0) + (hasCreateFromRoot ? 1 : 0) + (root ? 1 : 0) + fi}
				<button
					type="button"
					class={`${rowBase} ${isSelected(f.id) ? 'bg-surface-selected' : ''} ${keyArrowPos === forkIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
					onmouseenter={() => (keyArrowPos = forkIdx)}
					onclick={() => void pick(f.id)}
				>
					<GitFork size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{f.name}</span>
					{#if f.is_dev_workspace}
						<Badge color="indigo" small>dev</Badge>
					{/if}
				</button>
			{/each}
			{#if pendingFork && !creatingFork}
				<div
					class="px-3 py-1.5 text-xs text-primary flex flex-row gap-2 items-center text-left rounded-sm bg-surface-selected cursor-default"
				>
					<GitFork size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{pendingFork.name}</span>
					<span class="text-2xs text-tertiary italic shrink-0 ml-auto">New</span>
				</div>
			{/if}
		</div>
	{/snippet}
</DropdownV2>
