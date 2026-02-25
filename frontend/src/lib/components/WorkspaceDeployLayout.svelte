<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { Badge } from './common'
	import Row from './common/table/Row.svelte'
	import Tooltip from './Tooltip.svelte'
	import type { Snippet } from 'svelte'
	import type { Kind } from '$lib/utils_deployable'

	interface DeployableItem {
		key: string
		path: string
		kind: Kind
		triggerKind?: string
		[key: string]: unknown
	}

	interface Props {
		items: DeployableItem[]
		selectedItems: string[]
		selectablePredicate?: (item: DeployableItem) => boolean
		deploymentStatus: Record<string, { status: 'loading' | 'deployed' | 'failed'; error?: string }>
		allSelected?: boolean
		emptyMessage?: string

		// Snippets for customization
		header?: Snippet
		alerts?: Snippet
		itemSummary?: Snippet<[DeployableItem]>
		itemActions?: Snippet<[DeployableItem]>
		footer?: Snippet

		// Callbacks
		onToggleItem?: (item: DeployableItem) => void
		onSelectAll?: () => void
		onDeselectAll?: () => void
	}

	let {
		items,
		selectedItems,
		selectablePredicate = () => true,
		deploymentStatus,
		allSelected = false,
		emptyMessage = 'No items to deploy',
		header,
		alerts,
		itemSummary,
		itemActions,
		footer,
		onToggleItem,
		onSelectAll,
		onDeselectAll
	}: Props = $props()

	let selectableItems = $derived(items.filter(selectablePredicate))
	let hasSelectableItems = $derived(selectableItems.length > 0)
</script>

<div class="flex flex-col h-full">
	<!-- Header section -->
	{#if header}
		<div class="bg-surface">
			{@render header()}
		</div>
	{/if}

	<!-- Alerts section -->
	{#if alerts}
		{@render alerts()}
	{/if}

	{#if items.length > 0}
		<!-- Select all row -->
		<div class="px-4 py-2 flex items-center justify-between">
			<div
				class="flex items-center gap-2 text-secondary text-sm"
				class:opacity-50={!hasSelectableItems}
			>
				<input
					type="checkbox"
					disabled={!hasSelectableItems}
					checked={allSelected}
					onchange={allSelected ? onDeselectAll : onSelectAll}
					class="rounded max-w-4 w-full"
				/> Select all
			</div>
		</div>

		<!-- Items list -->
		<div class="overflow-y-auto">
			<div class="border rounded-md bg-surface-tertiary">
				{#each items as item (item.key)}
					{@const isSelectable = selectablePredicate(item)}
					{@const isSelected = selectedItems.includes(item.key)}
					{@const status = deploymentStatus[item.key]}
					{@const isDeployed = status?.status === 'deployed'}

					<Row
						isSelectable={isSelectable && !isDeployed}
						alignWithSelectable={true}
						disabled={!isSelectable}
						selected={isSelected && !isDeployed}
						onSelect={() => onToggleItem?.(item)}
						path={item.kind !== 'resource' &&
						item.kind !== 'variable' &&
						item.kind !== 'resource_type'
							? item.path
							: ''}
						marked={undefined}
						kind={item.kind}
						triggerKind={item.triggerKind}
						canFavorite={false}
						workspaceId=""
					>
						{#snippet customSummary()}
							{#if itemSummary}
								{@render itemSummary(item)}
							{:else}
								{item.path}
							{/if}
						{/snippet}
						{#snippet actions()}
							{#if itemActions}
								{@render itemActions(item)}
							{/if}
							<!-- Deployment status always shown -->
							{#if status}
								{#if status.status === 'loading'}
									<Loader2 class="animate-spin" />
								{:else if status.status === 'deployed'}
									<Badge color="green">Deployed</Badge>
								{:else if status.status === 'failed'}
									<div class="inline-flex gap-1">
										<Badge color="red">Failed</Badge>
										<Tooltip>{status.error}</Tooltip>
									</div>
								{/if}
							{/if}
						{/snippet}
					</Row>
				{/each}
			</div>
		</div>
	{:else}
		<div class="flex items-center justify-center h-full">
			<div class="text-gray-500">{emptyMessage}</div>
		</div>
	{/if}

	<!-- Footer -->
	{#if footer}
		<div class="p-4 bg-surface">
			{@render footer()}
		</div>
	{/if}
</div>
