<script lang="ts">
	import { userWorkspaces } from '$lib/stores'
	import { findWorkspaceRoot } from '$lib/utils/workspaceHierarchy'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { Badge, Button } from '$lib/components/common'
	import { Building, ChevronDown, GitFork } from 'lucide-svelte'

	// Shared chip that displays the workspace a fork picker points at. A fork (or a staged pending fork)
	// renders "<GitFork> <parent> → <fork>" with the fork accent-highlighted; a root renders
	// "<Building> <name>". Used as the trigger for both the sidebar workspace header and the session
	// "Run in" bar, so the two look identical — the only real difference between those consumers is how
	// they create forks.
	let {
		workspaceId,
		pendingFork,
		isCollapsed = false,
		showChevron = true,
		rootLabel = undefined,
		wrap = false,
		color = undefined,
		class: className = ''
	}: {
		workspaceId?: string
		pendingFork?: { id: string; name: string; parent_workspace_id: string }
		isCollapsed?: boolean
		// The dropdown-affordance chevron. Off for read-only displays (e.g. the started-session header).
		showChevron?: boolean
		// Replaces the root workspace's name with a muted, icon-less label (e.g.
		// "in root (2 forks)" in the sidebar, where the workspace menu right above
		// already shows the family name). Fork display is unaffected; the title
		// keeps the real name.
		rootLabel?: string
		// Two-row fork layout for narrow containers like the sidebar: "parent →" on
		// the first row, the fork name on its own full row (so it isn't sacrificed
		// to the parent). Each row truncates independently when even a full row is
		// too narrow. Root display is unaffected.
		wrap?: boolean
		// Overrides the fork accent color (used by the creation form's live
		// preview). Real forks read their workspace's `color` automatically.
		color?: string
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
	// The parent segment only appears for a fork of a fork — forking off the
	// root is the default and doesn't need spelling out.
	const shownParent = $derived(parentId !== root?.id ? parentName : undefined)

	// A fork with a workspace color re-themes the whole chip: forkAccentStyle
	// derives light/dark bg+text at that hue (matching the default blue's
	// profile) into CSS vars consumed by the accent classes below. Muted
	// segments (parent, arrow, "(new)") use opacity so they follow whichever
	// accent is active.
	const accentStyle = $derived(showFork ? forkAccentStyle(color ?? currentWs?.color) : undefined)
	const forkAccentClasses = $derived(
		accentStyle
			? 'bg-[color:var(--fork-accent-bg)] dark:bg-[color:var(--fork-accent-bg-dark)] text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)] font-semibold'
			: 'bg-surface-accent-selected text-accent font-semibold'
	)
</script>

<Button
	variant="subtle"
	unifiedSize="xs"
	title={showFork && shownParent ? `${shownParent} → ${name}` : name}
	style={accentStyle ?? ''}
	startIcon={isCollapsed
		? // Icon-only mode is the fork-picker affordance, whatever is selected.
			{ icon: GitFork }
		: !showFork && rootLabel
			? undefined
			: showFork && shownParent
				? // Fork-of-a-fork renders a fork icon inline before EACH name (the
					// parent is a fork too) — a third leading icon would be noise.
					undefined
				: { icon: showFork ? GitFork : Building }}
	endIcon={showChevron && !isCollapsed ? { icon: ChevronDown } : undefined}
	wrapperClasses={className}
	btnClasses="min-w-0 w-full rounded-md text-2xs {isCollapsed
		? 'justify-center'
		: 'justify-start'} {showFork ? forkAccentClasses : 'bg-surface-secondary'} {wrap
		? // The unified size's fixed h-5 would clip the two-row layout — let the
			// content set the height and breathe with vertical padding instead. The
			// min height keeps the single-line variant from looking squashed next
			// to the two-row one.
			'h-auto py-1 min-h-7'
		: ''}"
>
	{#if !isCollapsed}
		{#if showFork && shownParent}
			{#if wrap}
				<!-- Hierarchy shown tree-style: the fork row indents under its parent. -->
				<span class="min-w-0 flex-1 flex flex-col gap-0.5 text-left">
					<span class="flex items-center gap-1 min-w-0 opacity-60 font-normal">
						<GitFork size={12} class="shrink-0" />
						<span class="truncate">{shownParent}</span>
					</span>
					<span class="flex items-center gap-1 min-w-0 pl-3">
						<GitFork size={12} class="shrink-0" />
						<span class="truncate">{name}</span>
					</span>
				</span>
			{:else}
				<span class="min-w-0 flex-1 flex items-center gap-1 text-left">
					<GitFork size={12} class="shrink-0 opacity-60" />
					<span class="truncate opacity-60 font-normal">{shownParent}</span>
					<span class="shrink-0 opacity-40 font-normal">→</span>
					<GitFork size={12} class="shrink-0" />
					<span class="truncate">{name}</span>
				</span>
			{/if}
		{:else}
			<span class="truncate min-w-0 flex-1 text-left {!showFork && rootLabel ? 'text-hint' : ''}">
				{showFork ? name : (rootLabel ?? name)}
			</span>
		{/if}
		{#if pendingFork}
			<span class="shrink-0 opacity-70 font-normal">(new)</span>
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
