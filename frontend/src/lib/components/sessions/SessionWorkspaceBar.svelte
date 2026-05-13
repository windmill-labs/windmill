<script lang="ts">
	import { get } from 'svelte/store'
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		type Session
	} from './sessionState.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronLeft, GitFork, Plus } from 'lucide-svelte'

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

	let dropdownOpen = $state(false)
	let creatingFork = $state(false)
	let newForkName = $state('')
	let forkInput: HTMLInputElement | undefined = $state(undefined)

	function pick(id: string) {
		// Pre-send only: writes the pending pick. workspace_id stays
		// undefined until the user sends their first message. Switching
		// the global workspace ensures the session stays in scope —
		// otherwise picking a sibling fork drops it out of
		// $visibleWorkspaceIds and the session page goes blank.
		setSessionPendingWorkspace(session.id, id)
		if (id !== get(workspaceStore)) {
			switchWorkspace(id)
		}
		dropdownOpen = false
		creatingFork = false
	}

	function enterCreateMode() {
		creatingFork = true
		newForkName = ''
		// Focus on next tick — the input is mounted in the same dropdown re-render.
		requestAnimationFrame(() => forkInput?.focus())
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
		const prefixed = `${WM_FORK_PREFIX}${baseId}`
		// Defer the API call. The fork is materialised lazily by
		// commitSessionWorkspace at first user-message send, so abandoning
		// the draft doesn't leave an orphan fork behind. Routing/scope
		// stay on the parent root until commit.
		setSessionPendingFork(session.id, {
			parent_workspace_id: root.id,
			id: prefixed,
			name
		})
		if (root.id !== get(workspaceStore)) switchWorkspace(root.id)
		dropdownOpen = false
		creatingFork = false
		newForkName = ''
	}
</script>

<div class="flex flex-row items-center gap-1 py-1 px-1 mb-1 text-2xs text-secondary">
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
				<GitFork class="w-3 h-3 shrink-0" />
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
			<div
				class="bg-surface-tertiary dark:border w-56 origin-top-left rounded-lg shadow-lg focus:outline-none py-1"
			>
				{#if !creatingFork}
					<div class="flex flex-col">
						{#if root}
							<button
								type="button"
								onclick={() => pick(root.id)}
								class="px-3 py-1.5 text-xs text-primary hover:bg-surface-hover flex flex-row gap-2 items-center text-left rounded-sm"
							>
								<GitFork size={14} class="shrink-0 text-tertiary" />
								<span class="truncate">{root.name}</span>
								<span class="text-2xs text-tertiary shrink-0 ml-auto">root</span>
							</button>
						{/if}
						{#each forks as f (f.id)}
							<button
								type="button"
								onclick={() => pick(f.id)}
								class="px-3 py-1.5 text-xs text-primary hover:bg-surface-hover flex flex-row gap-2 items-center text-left rounded-sm"
							>
								<GitFork size={14} class="shrink-0 text-tertiary" />
								<span class="truncate">{f.name}</span>
							</button>
						{/each}
						<div class="my-1 border-t border-border-light"></div>
						<button
							type="button"
							onclick={enterCreateMode}
							class="px-3 py-1.5 text-xs text-primary hover:bg-surface-hover flex flex-row gap-2 items-center text-left rounded-sm"
						>
							<Plus size={14} class="shrink-0 text-tertiary" />
							<span>Create new fork…</span>
						</button>
					</div>
				{:else}
					<div class="flex flex-col px-2 py-1.5 gap-y-1">
						<div class="flex flex-row items-center gap-1">
							<Button
								variant="subtle"
								unifiedSize="xs"
								iconOnly
								startIcon={{ icon: ChevronLeft }}
								on:click={cancelCreate}
								title="Back"
							/>
							<span class="text-2xs font-normal text-secondary">New fork</span>
						</div>
						<!-- svelte-ignore a11y_autofocus -->
						<input
							bind:this={forkInput}
							type="text"
							bind:value={newForkName}
							placeholder="Fork name"
							autofocus
							onkeydown={(e) => {
								if (e.key === 'Enter') stageNewFork()
								else if (e.key === 'Escape') cancelCreate()
							}}
							class="w-full bg-surface-input border border-normal rounded px-1.5 py-1 text-xs font-normal text-primary outline-none focus:border-accent"
						/>
						<div class="flex flex-row justify-end gap-1">
							<Button variant="default" unifiedSize="xs" on:click={cancelCreate}>Cancel</Button>
							<Button
								variant="accent"
								unifiedSize="xs"
								on:click={stageNewFork}
								disabled={!newForkName.trim()}
							>
								Stage
							</Button>
						</div>
						<span class="text-2xs text-tertiary">Created when you send your first message.</span>
					</div>
				{/if}
			</div>
		{/snippet}
	</DropdownV2>
</div>
