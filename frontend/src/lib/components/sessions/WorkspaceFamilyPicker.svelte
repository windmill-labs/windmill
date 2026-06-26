<script lang="ts">
	import { tick, type Snippet } from 'svelte'
	import {
		enterpriseLicense,
		userWorkspaces,
		workspaceStore,
		type UserWorkspace
	} from '$lib/stores'
	import { findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import InputError from '$lib/components/InputError.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Building, Check, GitFork, Plus, Settings } from 'lucide-svelte'

	type PendingFork = { id: string; name: string }
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
		// Alternative to onCreateFork's inline staging: when set, the
		// "Create new fork…" row becomes a one-shot button that delegates to
		// this callback (e.g. opening the global fork modal) instead of
		// revealing the inline name input. Used where there's no pending draft
		// to stage (the sidebar scope picker).
		onRequestCreateFork,
		allowCreateFork = true,
		// Optional caption rendered under the new-fork input. Lets the
		// consumer differentiate "staged for first send" vs. "will be
		// created immediately" semantics.
		createForkCaption = '',
		// Forwarded to the dropdown trigger wrapper — e.g. `min-w-0` so the
		// trigger can shrink and its label truncate inside a narrow container.
		class: triggerClass = undefined,
		// Optional settings link rendered at the very bottom of the menu (e.g.
		// "<workspace> settings"). The consumer decides visibility (admin gate)
		// and target href.
		settingsHref,
		settingsLabel,
		trigger
	}: {
		selectedId?: string
		pendingFork?: PendingFork
		onPick: (workspaceId: string) => void | Promise<void>
		onCreateFork?: (fork: ForkRequest) => void | Promise<void>
		onRequestCreateFork?: () => void
		allowCreateFork?: boolean
		createForkCaption?: string
		class?: string
		settingsHref?: string
		settingsLabel?: string
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

	// Structural gate, same as the sidebar WorkspaceMenu / SessionWorkspaceBar:
	// when closed (cloud / DisableWorkspaceForking rule / admins workspace) the
	// fork affordance is hidden entirely.
	const forksGateOpen = $derived(
		!isCloudHosted() && !isRuleActive('DisableWorkspaceForking') && $workspaceStore !== 'admins'
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
	const forkAffordanceOpen = $derived(
		allowCreateFork && forksGateOpen && (!!onCreateFork || !!onRequestCreateFork) && !!root
	)
	// The upsell (CE workspace cap) only applies to in-place inline creation;
	// onRequestCreateFork delegates to a flow that enforces its own limits.
	const showCreateFork = $derived(
		forkAffordanceOpen && (!ceWorkspaceCapReached || !!onRequestCreateFork)
	)
	const showForkUpsell = $derived(
		forkAffordanceOpen && ceWorkspaceCapReached && !onRequestCreateFork
	)

	let dropdownOpen = $state(false)
	let creatingFork = $state(false)
	let newForkName = $state('')
	let forkInput: TextInput | undefined = $state(undefined)

	// Manual keyboard navigation, modelled after SelectDropdown. melt's
	// menu API couples Enter/Space to closing the menu, which we explicitly
	// don't want for the "Create new fork" row — it swaps to inline input.
	type NavRow = { kind: 'create' } | { kind: 'root'; id: string } | { kind: 'fork'; id: string }
	const navRows = $derived<NavRow[]>([
		...(showCreateFork ? [{ kind: 'create' as const }] : []),
		...(root ? [{ kind: 'root' as const, id: root.id }] : []),
		...forks.map((f) => ({ kind: 'fork' as const, id: f.id }))
	])
	let keyArrowPos = $state<number | undefined>(undefined)
	$effect(() => {
		if (!dropdownOpen) keyArrowPos = undefined
	})

	function activateRow(row: NavRow) {
		if (row.kind === 'create') {
			requestCreateFork()
		} else if (row.kind === 'root' || row.kind === 'fork') {
			void pick(row.id)
		}
	}

	// Delegated-create mode (onRequestCreateFork) closes the dropdown and hands
	// off; inline mode reveals the name input.
	function requestCreateFork() {
		if (onRequestCreateFork) {
			dropdownOpen = false
			creatingFork = false
			onRequestCreateFork()
		} else {
			void enterCreateMode()
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

	async function enterCreateMode(initialName?: string) {
		creatingFork = true
		newForkName = initialName ?? defaultForkName()
		await tick()
		forkInput?.focus()
		forkInput?.select()
	}

	function cancelCreate() {
		creatingFork = false
		newForkName = ''
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
		if (!root || !name || forkNameError || !onCreateFork) return
		const baseId = slugForkBaseId(name)
		if (!baseId) return
		const prefixed = `${WM_FORK_PREFIX}${baseId}`
		// Close optimistically; consumer can re-open + toast on error.
		creatingFork = false
		newForkName = ''
		dropdownOpen = false
		await onCreateFork({ parent_workspace_id: root.id, id: prefixed, name })
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
			void enterCreateMode(pendingFork.name)
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
	class={triggerClass}
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
						onclick={() => requestCreateFork()}
					>
						<Plus size={14} class="shrink-0 text-tertiary" />
						<span>Create new fork…</span>
					</button>
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
					<span>Create new fork…</span>
					<span class="ml-auto shrink-0 text-2xs text-tertiary"> Workspace limit reached </span>
				</div>
				<div class="my-1 border-t border-border-light shrink-0"></div>
			{/if}

			{#if root}
				{@const rootIdx = showCreateFork ? 1 : 0}
				<button
					type="button"
					class={`${rowBase} ${isSelected(root.id) && !pendingFork ? 'bg-surface-selected' : ''} ${keyArrowPos === rootIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
					onmouseenter={() => (keyArrowPos = rootIdx)}
					onclick={() => void pick(root.id)}
				>
					<Building size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{root.name}</span>
					<span class="text-2xs text-tertiary shrink-0 ml-auto">root</span>
				</button>
			{/if}
			{#each forks as f, fi (f.id)}
				{@const forkIdx = (showCreateFork ? 1 : 0) + (root ? 1 : 0) + fi}
				<button
					type="button"
					class={`${rowBase} ${isSelected(f.id) ? 'bg-surface-selected' : ''} ${keyArrowPos === forkIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
					onmouseenter={() => (keyArrowPos = forkIdx)}
					onclick={() => void pick(f.id)}
				>
					<GitFork size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{f.name}</span>
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
			{#if settingsHref}
				<div class="my-1 border-t border-border-light shrink-0"></div>
				<a
					href={settingsHref}
					onclick={() => (dropdownOpen = false)}
					class={`${rowBase} hover:bg-surface-hover text-secondary`}
				>
					<Settings size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{settingsLabel ?? 'Workspace settings'}</span>
				</a>
			{/if}
		</div>
	{/snippet}
</DropdownV2>
