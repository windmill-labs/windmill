<script lang="ts">
	import { GitFork, ChevronUp, ArchiveRestore } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { Button } from '$lib/components/common'
	import type { UserWorkspace } from '$lib/stores'
	import { superadmin } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { pluralize } from '$lib/utils'
	import WorkspaceIcon from './WorkspaceIcon.svelte'
	import WorkspaceCard from './WorkspaceCard.svelte'
	import { twMerge } from 'tailwind-merge'

	interface ExtendedWorkspace extends UserWorkspace {
		_children?: ExtendedWorkspace[]
		marked?: string
	}

	interface Props {
		workspace: UserWorkspace & { marked?: string }
		isForked?: boolean
		depth?: number
		children?: ExtendedWorkspace[]
		isExpanded?: boolean
		expansionStates?: Record<string, boolean>
		onEnterWorkspace: (workspaceId: string) => Promise<void>
		onUnarchive?: (workspaceId: string) => Promise<void>
		onToggleExpand?: (workspaceId: string) => void
	}

	let {
		workspace,
		isForked = false,
		depth = 0,
		children = [],
		isExpanded = false,
		expansionStates = {},
		onEnterWorkspace,
		onUnarchive,
		onToggleExpand
	}: Props = $props()

	const paddingLeft = depth * 24

	// Helper functions
	function isWorkspaceArchived(workspace: UserWorkspace): boolean {
		return workspace['deleted'] === true
	}

	function isWorkspaceDisabled(workspace: UserWorkspace): boolean {
		return workspace.disabled === true
	}

	async function handleUnarchive() {
		if (onUnarchive) {
			await WorkspaceService.unarchiveWorkspace({ workspace: workspace.id })
			await onUnarchive(workspace.id)
		}
	}
</script>

<div class="block pb-2" style:padding-left={`${paddingLeft}px`}>
	<div class="border border-border-light rounded-lg bg-surface-tertiary overflow-hidden">
		<!-- Main workspace card - clickable to enter workspace -->
		<div
			class="px-4 py-2 hover:bg-surface-hover transition-colors cursor-pointer w-full"
			class:rounded-lg={children.length === 0}
			class:rounded-b-none={children.length > 0}
			role="button"
			tabindex="0"
			onclick={async () => {
				if (!isWorkspaceDisabled(workspace)) {
					await onEnterWorkspace(workspace.id)
				}
			}}
			onkeydown={(e) => {
				if ((e.key === 'Enter' || e.key === ' ') && !isWorkspaceDisabled(workspace)) {
					e.preventDefault()
					onEnterWorkspace(workspace.id)
				}
			}}
			class:opacity-60={isWorkspaceDisabled(workspace)}
			class:cursor-not-allowed={isWorkspaceDisabled(workspace)}
			class:cursor-pointer={!isWorkspaceDisabled(workspace)}
		>
			<div class="flex flex-row items-center justify-between">
				<div class="flex flex-row items-center gap-3 flex-1 min-w-0">
					<div class="flex flex-row items-center gap-2 flex-1 min-w-0">
						<div class="flex-shrink-0">
							<WorkspaceIcon
								workspaceColor={workspace.color}
								{isForked}
								parentName={workspace.parent_workspace_id ?? undefined}
								size={12}
							/>
						</div>

						<div class="min-w-0 flex-1">
							<div class="flex flex-row items-center gap-2 flex-wrap">
								<span class="text-xs font-semibold text-primary truncate">
									{#if workspace.marked}
										{@html workspace.marked}
									{:else}
										{workspace.name}
									{/if}
								</span>
								<span class="text-secondary text-xs">-</span>
								<span class="font-mono text-2xs text-secondary truncate">
									{workspace.id}
								</span>
							</div>

							<div class="text-xs text-secondary">
								as <span class="font-mono">{workspace.username}</span>
								{#if isWorkspaceArchived(workspace)}
									<span class="text-red-500 ml-1">(archived)</span>
									{#if $superadmin && onUnarchive}
										<Button
											size="xs2"
											variant="default"
											btnClasses="ml-1"
											propagateEvent={false}
											onClick={handleUnarchive}
											startIcon={{ icon: ArchiveRestore }}
										>
											Unarchive
										</Button>
									{/if}
								{/if}
								{#if isWorkspaceDisabled(workspace)}
									<span class="text-red-500 ml-1">(user disabled in this workspace)</span>
								{/if}
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Forks section - clickable to expand -->
		{#if children.length > 0}
			<div
				class="border-t border-border-light px-4 py-1 hover:bg-surface-hover transition-colors cursor-pointer"
				role="button"
				tabindex="0"
				onclick={() => onToggleExpand?.(workspace.id)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault()
						onToggleExpand?.(workspace.id)
					}
				}}
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
			{#each children as child (child.id)}
				<WorkspaceCard
					workspace={child}
					isForked={true}
					depth={depth + 1}
					children={child._children || []}
					isExpanded={expansionStates[child.id] ?? false}
					{expansionStates}
					{onEnterWorkspace}
					{onUnarchive}
					{onToggleExpand}
				/>
			{/each}
		</div>
	{/if}
</div>
