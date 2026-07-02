<script lang="ts">
	import { userWorkspaces } from '$lib/stores'
	import { findWorkspaceRoot } from '$lib/utils/workspaceHierarchy'
	import { Badge, Button } from '$lib/components/common'
	import { Building, ChevronDown, GitFork } from 'lucide-svelte'

	// Shared chip that displays the workspace a fork picker points at. A fork (or a staged pending fork)
	// renders "<GitFork> <fork> → <parent>" with accent highlighting; a root renders "<Building> <name>".
	// Used as the trigger for both the sidebar workspace header and the session "Run in" bar, so the two
	// look identical — the only real difference between those consumers is how they create forks.
	let {
		workspaceId,
		pendingFork,
		isCollapsed = false,
		showChevron = true,
		rootLabel = undefined,
		class: className = ''
	}: {
		workspaceId?: string
		pendingFork?: { id: string; name: string; parent_workspace_id: string }
		isCollapsed?: boolean
		// The dropdown-affordance chevron. Off for read-only displays (e.g. the started-session header).
		showChevron?: boolean
		// Replaces the root workspace's name with a muted, icon-less label (e.g.
		// "workspace root" in the sidebar, where the workspace menu right above
		// already shows the family name). Fork display is unaffected; the title
		// keeps the real name.
		rootLabel?: string
		class?: string
	} = $props()

	const root = $derived(findWorkspaceRoot(workspaceId, $userWorkspaces))
	const currentWs = $derived(
		workspaceId ? $userWorkspaces.find((w) => w.id === workspaceId) : undefined
	)
	const isFork = $derived(!!currentWs && !!root && currentWs.id !== root.id)
	const showFork = $derived(!!pendingFork || isFork)
	const name = $derived(pendingFork?.name ?? currentWs?.name ?? workspaceId ?? 'Pick workspace')
	const parentId = $derived(pendingFork?.parent_workspace_id ?? currentWs?.parent_workspace_id)
	const parentName = $derived(
		parentId ? ($userWorkspaces.find((w) => w.id === parentId)?.name ?? parentId) : undefined
	)
</script>

<Button
	variant="subtle"
	unifiedSize="xs"
	title={showFork && parentName ? `${name} → ${parentName}` : name}
	startIcon={isCollapsed
		? // Icon-only mode is the fork-picker affordance, whatever is selected.
			{ icon: GitFork }
		: !showFork && rootLabel
			? undefined
			: { icon: showFork ? GitFork : Building }}
	endIcon={showChevron && !isCollapsed ? { icon: ChevronDown } : undefined}
	wrapperClasses={className}
	btnClasses="min-w-0 w-full rounded-md text-2xs {isCollapsed
		? 'justify-center'
		: 'justify-start'} {showFork
		? 'bg-surface-accent-selected text-accent font-semibold'
		: 'bg-surface-secondary'}"
>
	{#if !isCollapsed}
		<span class="truncate min-w-0 flex-1 text-left {!showFork && rootLabel ? 'text-hint' : ''}">
			{showFork ? name : (rootLabel ?? name)}
			{#if showFork && parentName}
				<span class="text-accent/40 font-normal">→</span>
				<span class="text-accent/60 font-normal">{parentName}</span>
			{/if}
		</span>
		{#if pendingFork}
			<span class="shrink-0 text-accent/70 font-normal">(new)</span>
		{:else if currentWs?.is_dev_workspace}
			<Badge
				color="dark-blue"
				small
				class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white"
				>dev</Badge
			>
		{/if}
	{/if}
</Button>
