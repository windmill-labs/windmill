<script lang="ts">
	import { untrack } from 'svelte'
	import { GitFork, ChevronUp, ArchiveRestore } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { Badge, Button } from '$lib/components/common'
	import type { UserWorkspace } from '$lib/stores'
	import { superadmin } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { reconcileAfterWorkspaceChange } from '$lib/components/sessions/sessionState.svelte'
	import { pluralize } from '$lib/utils'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import WorkspaceIcon from './WorkspaceIcon.svelte'
	import WorkspaceCard from './WorkspaceCard.svelte'
	import { twMerge } from 'tailwind-merge'
	import { devBadgeText, devLabelWord } from '$lib/utils/devWorkspaceLabel'

	interface ExtendedWorkspace extends UserWorkspace {
		_children?: ExtendedWorkspace[]
		_dev?: ExtendedWorkspace
		marked?: string
	}

	interface Props {
		workspace: UserWorkspace & { marked?: string }
		dev?: ExtendedWorkspace
		isForked?: boolean
		depth?: number
		children?: ExtendedWorkspace[]
		isExpanded?: boolean
		expansionStates?: Record<string, boolean>
		onEnterWorkspace: (workspaceId: string) => Promise<void>
		onUnarchive?: (workspaceId: string) => Promise<void>
		onToggleExpand?: (workspaceId: string) => void
		selectedWorkspaceId?: string | null
		onMouseEnter?: (workspaceId: string) => void
		onMouseClick?: () => void
		onKeyboardNavigation?: () => void
	}

	let {
		workspace,
		dev,
		isForked = false,
		depth = 0,
		children = [],
		isExpanded = false,
		expansionStates = {},
		onEnterWorkspace,
		onUnarchive,
		onToggleExpand,
		selectedWorkspaceId,
		onMouseEnter,
		onMouseClick,
		onKeyboardNavigation
	}: Props = $props()

	const paddingLeft = untrack(() => depth) * 24
	const isSelected = $derived(selectedWorkspaceId === workspace.id)
	// Colored forks render icon + name in the derived fork accent (the fork
	// picker convention); the icon side is handled inside WorkspaceIcon.
	const forkAccent = $derived(isForked ? forkAccentStyle(workspace.color) : undefined)

	// A dev workspace only reaches the fork list when it hangs off another dev workspace; it still
	// sorts before throwaway forks.
	const sortedChildren = $derived(
		[...children].sort((a, b) => {
			if (!!a.is_dev_workspace !== !!b.is_dev_workspace) return a.is_dev_workspace ? -1 : 1
			return a.name.localeCompare(b.name)
		})
	)

	// With a dev workspace present the two environment cards below own the choice, so the header
	// stops being an entry point of its own.
	const headerEntersWorkspace = $derived(dev === undefined)
	const headerClass = $derived(
		twMerge(
			'px-4 py-2 transition-colors w-full',
			children.length === 0 ? 'rounded-lg' : 'rounded-b-none'
		)
	)

	// Helper functions
	function isWorkspaceArchived(workspace: UserWorkspace): boolean {
		return workspace['deleted'] === true
	}

	function isWorkspaceDisabled(workspace: UserWorkspace): boolean {
		return workspace.disabled === true
	}

	async function handleUnarchive(target: UserWorkspace) {
		if (onUnarchive) {
			await WorkspaceService.unarchiveWorkspace({ workspace: target.id })
			await onUnarchive(target.id)
			// Restore sessions auto-archived when this workspace was archived.
			await reconcileAfterWorkspaceChange()
		}
	}

	function enter(target: UserWorkspace) {
		onMouseClick?.()
		if (!isWorkspaceDisabled(target)) {
			onEnterWorkspace(target.id)
		}
	}

	function enterOnKey(e: KeyboardEvent, target: UserWorkspace) {
		if ((e.key === 'Enter' || e.key === ' ') && !isWorkspaceDisabled(target)) {
			e.preventDefault()
			onKeyboardNavigation?.()
			onEnterWorkspace(target.id)
		}
	}
</script>

{#snippet header()}
	<div class="flex flex-row items-center justify-between">
		<div class="flex flex-row items-center gap-3 flex-1 min-w-0">
			<div class="flex flex-row items-center gap-2 flex-1 min-w-0">
				<div class="flex-shrink-0">
					<WorkspaceIcon
						workspaceColor={workspace.color}
						{isForked}
						isDevWorkspace={workspace.is_dev_workspace}
						devWorkspaceLabel={workspace.dev_workspace_label}
						parentName={workspace.parent_workspace_id ?? undefined}
						size={12}
					/>
				</div>

				<div class="min-w-0 flex-1">
					<div class="flex flex-row items-center gap-2 flex-wrap">
						<span
							class="text-xs font-semibold truncate {forkAccent
								? 'text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]'
								: 'text-primary'}"
							style={forkAccent}
						>
							{#if workspace.marked}
								{@html workspace.marked}
							{:else}
								{workspace.name}
							{/if}
						</span>
						{#if workspace.is_dev_workspace}
							<Badge
								color="dark-blue"
								small
								class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white"
								>{devBadgeText(workspace.dev_workspace_label)}</Badge
							>
						{/if}
						<span class="text-secondary text-xs">-</span>
						{#if workspace.id === 'admins'}
							<Badge color="blue">{workspace.id}</Badge>
						{:else}
							<span class="font-mono text-2xs text-secondary truncate">
								{workspace.id}
							</span>
						{/if}
					</div>

					{#if headerEntersWorkspace}
						{@render identity(workspace)}
					{/if}
				</div>
			</div>
		</div>
	</div>
{/snippet}

{#snippet identity(target: UserWorkspace & { marked?: string })}
	<div class="text-xs text-secondary">
		as <span class="font-mono">{target.username}</span>
		{#if isWorkspaceArchived(target)}
			<span class="text-red-500 ml-1">(archived)</span>
			{#if $superadmin && onUnarchive}
				<Button
					size="xs2"
					variant="default"
					btnClasses="ml-1"
					propagateEvent={false}
					onClick={() => handleUnarchive(target)}
					startIcon={{ icon: ArchiveRestore }}
				>
					Unarchive
				</Button>
			{/if}
		{/if}
		{#if isWorkspaceDisabled(target)}
			<span class="text-red-500 ml-1">(user disabled in this workspace)</span>
		{/if}
		{#if target.id === 'admins'}
			<span class="text-accent ml-1">Used to manage your Windmill instance</span>
		{/if}
	</div>
{/snippet}

<!-- One half of the prod/dev pair: the two environments are siblings, so each carries its own
     entry affordance and the card header above them is inert. -->
{#snippet environment(target: ExtendedWorkspace, label: string)}
	<div
		class={twMerge(
			'flex-1 min-w-0 border border-border-light rounded-md px-3 py-2 transition-colors',
			selectedWorkspaceId === target.id ? 'bg-surface-hover' : 'bg-surface',
			isWorkspaceDisabled(target)
				? 'opacity-60 cursor-not-allowed'
				: 'cursor-pointer hover:bg-surface-hover'
		)}
		data-workspace-id={target.id}
		role="button"
		tabindex="0"
		onclick={() => enter(target)}
		onkeydown={(e) => enterOnKey(e, target)}
		onmouseenter={() => onMouseEnter?.(target.id)}
	>
		<div class="flex flex-row items-center gap-2 min-w-0">
			<span class="text-xs font-semibold text-primary truncate">{label}</span>
			{#if target.id !== workspace.id}
				<span class="font-mono text-2xs text-secondary truncate">
					{#if target.marked}
						{@html target.marked}
					{:else}
						{target.id}
					{/if}
				</span>
			{/if}
		</div>
		{@render identity(target)}
	</div>
{/snippet}

<div class="block pb-2" style:padding-left={`${paddingLeft}px`}>
	<div
		class={twMerge(
			'border border-border-light rounded-md overflow-hidden transition-all duration-150',
			isSelected && headerEntersWorkspace ? 'bg-surface-hover' : 'bg-surface-tertiary'
		)}
		data-workspace-id={workspace.id}
	>
		{#if headerEntersWorkspace}
			<!-- Main workspace card - clickable to enter workspace -->
			<div
				class={twMerge(
					headerClass,
					'hover:bg-surface-hover',
					isWorkspaceDisabled(workspace) ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'
				)}
				role="button"
				tabindex="0"
				onclick={() => enter(workspace)}
				onkeydown={(e) => enterOnKey(e, workspace)}
				onmouseenter={() => onMouseEnter?.(workspace.id)}
			>
				{@render header()}
			</div>
		{:else}
			<div class={headerClass}>
				{@render header()}
			</div>
		{/if}

		{#if dev}
			<div class="flex flex-row gap-2 px-4 pb-3">
				{@render environment(workspace, 'Production')}
				{@render environment(dev, devLabelWord(dev.dev_workspace_label))}
			</div>
		{/if}

		<!-- Forks section - clickable to expand -->
		{#if children.length > 0}
			<div
				class="border-t border-border-light px-4 py-1.5 hover:bg-surface-hover transition-colors cursor-pointer"
				role="button"
				tabindex="0"
				onclick={() => {
					onMouseClick?.()
					onToggleExpand?.(workspace.id)
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault()
						onKeyboardNavigation?.()
						onToggleExpand?.(workspace.id)
					}
				}}
				onmouseenter={() => onMouseEnter?.(workspace.id)}
			>
				<div class="flex flex-row items-center justify-between">
					<div class="flex flex-row items-center gap-2 pl-2">
						<GitFork size={10} class="text-primary" />
						<span class="text-2xs text-primary">
							{pluralize(children.length, 'fork', 'forks')}
						</span>
					</div>
					<div class="flex items-center">
						<ChevronUp
							size={16}
							class={twMerge(
								'text-secondary transition-transform duration-150',
								isExpanded ? 'transform rotate-180' : ''
							)}
						/>
					</div>
				</div>
			</div>
		{/if}
	</div>

	<!-- Expanded forks -->
	{#if children.length > 0 && isExpanded}
		<div class="mt-2 ml-6" transition:slide={{ duration: 150 }}>
			{#each sortedChildren as child (child.id)}
				<WorkspaceCard
					workspace={child}
					dev={child._dev}
					isForked={true}
					depth={depth + 1}
					children={child._children || []}
					isExpanded={expansionStates[child.id] ?? false}
					{expansionStates}
					{onEnterWorkspace}
					{onUnarchive}
					{onToggleExpand}
					{selectedWorkspaceId}
					{onMouseEnter}
					{onMouseClick}
					{onKeyboardNavigation}
				/>
			{/each}
		</div>
	{/if}
</div>
