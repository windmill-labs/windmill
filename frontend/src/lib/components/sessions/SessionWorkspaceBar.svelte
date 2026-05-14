<script lang="ts">
	import { tick } from 'svelte'
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		syncWorkspaceTo,
		type Session
	} from './sessionState.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Building, Check, ChevronDown, GitFork, Plus } from 'lucide-svelte'

	let { session }: { session: Session } = $props()

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

	// Effective workspace for display: committed → pending pick → active store.
	const effectiveId = $derived(getEffectiveWorkspaceId(session) ?? $workspaceStore ?? undefined)
	const root = $derived(findRoot(effectiveId, $userWorkspaces))
	const forks = $derived(root ? findWorkspaceDescendants(root.id, $userWorkspaces) : [])
	const currentWs = $derived(
		effectiveId ? $userWorkspaces.find((w) => w.id === effectiveId) : undefined
	)
	const pendingFork = $derived(session.pending_fork)

	// Same gate as the sidebar WorkspaceMenu.
	const forksAllowed = $derived(
		!isCloudHosted() && !isRuleActive('DisableWorkspaceForking') && $workspaceStore !== 'admins'
	)

	let dropdownOpen = $state(false)
	let creatingFork = $state(false)
	let newForkName = $state('')
	let forkInput: HTMLInputElement | undefined = $state(undefined)

	// Manual keyboard navigation, modelled after SelectDropdown.svelte. Melt's
	// menu API couples Enter/Space to closing the menu, which we explicitly
	// don't want for the "Create new fork" row — it swaps to inline input.
	// Index 0 = Create-fork row (when visible), 1 = root, 2..N+1 = descendant forks.
	type NavRow = { kind: 'create' } | { kind: 'root'; id: string } | { kind: 'fork'; id: string }
	const navRows = $derived<NavRow[]>([
		...(forksAllowed && root ? [{ kind: 'create' as const }] : []),
		...(root ? [{ kind: 'root' as const, id: root.id }] : []),
		...forks.map((f) => ({ kind: 'fork' as const, id: f.id }))
	])
	let keyArrowPos = $state<number | undefined>(undefined)
	$effect(() => {
		if (!dropdownOpen) keyArrowPos = undefined
	})

	// Reopening the dropdown while a pending fork is staged drops the user
	// directly into edit mode (input pre-filled with the staged name) so
	// they can refine it before sending. Tracks the open transition to
	// avoid re-entering edit mode after the user explicitly cancelled.
	let lastDropdownOpen = $state(false)
	$effect(() => {
		const wasOpen = lastDropdownOpen
		lastDropdownOpen = dropdownOpen
		if (dropdownOpen && !wasOpen && session.pending_fork && !creatingFork) {
			void enterCreateMode(session.pending_fork.name)
		}
	})

	function activate(row: NavRow) {
		if (row.kind === 'create') {
			void enterCreateMode()
		} else if (row.kind === 'root' || row.kind === 'fork') {
			pick(row.id)
		}
	}

	// Pick a human-friendly default name shaped like "<adjective>-fork"
	// (e.g. "zippy-fork"). Same adjective pool Path.svelte uses for new
	// scripts/flows. Tries a handful of random picks to avoid colliding
	// with an existing workspace or the session's own pending fork; falls
	// back to `<adj>-fork-N` if every attempt collided.
	function defaultForkName(): string {
		const taken = new Set($userWorkspaces.map((w) => w.id))
		if (session.pending_fork) taken.add(session.pending_fork.id)
		for (let i = 0; i < 50; i++) {
			const name = `${random_adj()}-fork`
			if (!taken.has(`${WM_FORK_PREFIX}${name}`)) return name
		}
		const base = `${random_adj()}-fork`
		let n = 1
		while (taken.has(`${WM_FORK_PREFIX}${base}-${n}`)) n++
		return `${base}-${n}`
	}

	function pick(id: string) {
		// Pre-send only: writes the pending pick. workspace_id stays
		// undefined until the user sends their first message.
		setSessionPendingWorkspace(session.id, id)
		syncWorkspaceTo(id)
		dropdownOpen = false
		creatingFork = false
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

	function stageNewFork() {
		const name = newForkName.trim()
		if (!root || !name) return
		const baseId = name
			.toLowerCase()
			.replace(/[^a-z0-9-]/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '')
		if (!baseId) return
		const prefixed = `${WM_FORK_PREFIX}${baseId}`
		setSessionPendingFork(session.id, {
			parent_workspace_id: root.id,
			id: prefixed,
			name
		})
		syncWorkspaceTo(root.id)
		creatingFork = false
		newForkName = ''
	}

	function isSelected(id: string): boolean {
		if (pendingFork?.id === id) return true
		return !pendingFork && effectiveId === id
	}
</script>

<svelte:window
	onkeydown={(e) => {
		// Only navigate while the dropdown is visible and the user isn't
		// typing into the inline create-fork input. Escape closes.
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
			activate(navRows[keyArrowPos])
			e.preventDefault()
		} else if (e.key === 'Escape') {
			dropdownOpen = false
			e.preventDefault()
		}
	}}
/>

<div class="flex flex-row items-center gap-1 py-0.5 px-1 text-2xs text-secondary">
	<span class="shrink-0">Run in</span>
	<DropdownV2
		bind:open={dropdownOpen}
		customMenu
		placement="bottom-start"
		fixedHeight={false}
		usePointerDownOutside
	>
		{#snippet buttonReplacement()}
			<span
				class="inline-flex flex-row items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-hover text-2xs"
			>
				{#if pendingFork || (currentWs && currentWs.id !== root?.id)}
					<GitFork class="w-3 h-3 shrink-0" />
				{:else}
					<Building class="w-3 h-3 shrink-0" />
				{/if}
				<span class="font-medium text-primary truncate max-w-[180px]">
					{pendingFork?.name ?? currentWs?.name ?? effectiveId ?? 'Pick workspace'}
				</span>
				{#if pendingFork}
					<span class="text-2xs text-tertiary italic shrink-0">(new)</span>
				{/if}
				<ChevronDown class="w-3 h-3 shrink-0 text-tertiary" />
			</span>
		{/snippet}
		{#snippet menu()}
			{@const rowBase =
				'px-3 py-1.5 text-xs text-primary flex flex-row gap-2 items-center text-left rounded-sm w-full'}
			<div
				class="bg-surface-tertiary dark:border w-64 origin-top-left rounded-lg shadow-lg focus:outline-none py-1 flex flex-col"
			>
				{#if forksAllowed && root}
					{#if creatingFork}
						<div class="flex flex-col gap-1 px-2 py-1.5">
							<div class="flex flex-row items-center gap-1.5">
								<Plus size={14} class="shrink-0 text-tertiary" />
								<!-- svelte-ignore a11y_autofocus -->
								<input
									bind:this={forkInput}
									type="text"
									bind:value={newForkName}
									placeholder="Fork name"
									autofocus
									onkeydown={(e) => {
										if (e.key === 'Enter') {
											e.preventDefault()
											e.stopPropagation()
											stageNewFork()
										} else if (e.key === 'Escape') {
											e.preventDefault()
											e.stopPropagation()
											cancelCreate()
										}
									}}
									class="flex-1 min-w-0 bg-surface-input border border-normal rounded px-1.5 py-0.5 text-xs font-normal text-primary outline-none focus:border-accent"
								/>
								<button
									type="button"
									aria-label="Confirm"
									title="Stage fork"
									onclick={stageNewFork}
									disabled={!newForkName.trim()}
									class="inline-flex items-center justify-center w-5 h-5 rounded text-accent hover:bg-surface-hover disabled:opacity-40 disabled:cursor-not-allowed"
								>
									<Check size={14} />
								</button>
							</div>
							<span class="text-2xs text-tertiary pl-6">
								Created when you send your first message.
							</span>
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
							<span>Create new fork…</span>
						</button>
					{/if}
					<div class="my-1 border-t border-border-light"></div>
				{/if}

				{#if root}
					{@const rootIdx = forksAllowed ? 1 : 0}
					<button
						type="button"
						class={`${rowBase} ${isSelected(root.id) && !pendingFork ? 'bg-surface-selected' : ''} ${keyArrowPos === rootIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
						onmouseenter={() => (keyArrowPos = rootIdx)}
						onclick={() => pick(root.id)}
					>
						<Building size={14} class="shrink-0 text-tertiary" />
						<span class="truncate">{root.name}</span>
						<span class="text-2xs text-tertiary shrink-0 ml-auto">root</span>
					</button>
				{/if}
				{#each forks as f, fi (f.id)}
					{@const forkIdx = (forksAllowed ? 1 : 0) + (root ? 1 : 0) + fi}
					<button
						type="button"
						class={`${rowBase} ${isSelected(f.id) ? 'bg-surface-selected' : ''} ${keyArrowPos === forkIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
						onmouseenter={() => (keyArrowPos = forkIdx)}
						onclick={() => pick(f.id)}
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
			</div>
		{/snippet}
	</DropdownV2>
</div>
