<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { Badge } from './common'
	import Checkbox from './common/checkbox/Checkbox.svelte'
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
		/** For a non-deployed item that isn't selectable, return a reason string to
		 * render a disabled checkbox + hover tooltip (instead of greying the row);
		 * return undefined to keep the default greyed-out, no-checkbox treatment. */
		selectBlockedReason?: (item: DeployableItem) => string | undefined
		deploymentStatus: Record<string, { status: 'loading' | 'deployed' | 'failed'; error?: string }>
		allSelected?: boolean
		emptyMessage?: string
		hideSelection?: boolean
		children?: Snippet

		// Snippets for customization
		header?: Snippet
		alerts?: Snippet
		/** Rendered on the right of the "Select all" row (e.g. a filter toggle). */
		selectAllActions?: Snippet
		itemSummary?: Snippet<[DeployableItem]>
		/** Overrides the secondary path line per item (e.g. to strike a renamed path). */
		itemPath?: Snippet<[DeployableItem]>
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
		selectBlockedReason,
		deploymentStatus,
		allSelected = false,
		emptyMessage = 'No items to deploy',
		hideSelection = false,
		header,
		alerts,
		selectAllActions,
		itemSummary,
		itemPath,
		itemActions,
		footer,
		onToggleItem,
		onSelectAll,
		onDeselectAll
	}: Props = $props()

	let selectableItems = $derived(items.filter(selectablePredicate))
	let hasSelectableItems = $derived(selectableItems.length > 0)

	// Plain row click and the checkbox both toggle this row in/out — multi-select
	// is the default, no modifier needed.
	function handleSelect(item: DeployableItem) {
		onToggleItem?.(item)
	}
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

	<!-- Controls row: "Select all" (when there are items and selection is enabled) +
	     optional right-side actions (e.g. a filter toggle). Renders when there are
	     items OR actions are provided, so a filter that empties the list doesn't take
	     its own toggle with it. -->
	{#if items.length > 0 || selectAllActions}
		<div class="px-4 py-2 flex items-center justify-between">
			{#if items.length > 0 && !hideSelection}
				<label
					class="flex items-center gap-2 text-secondary text-xs"
					class:opacity-50={!hasSelectableItems}
					class:cursor-pointer={hasSelectableItems}
				>
					<Checkbox
						disabled={!hasSelectableItems}
						checked={allSelected}
						onChange={allSelected ? onDeselectAll : onSelectAll}
					/> Select all
				</label>
			{:else}
				<span></span>
			{/if}
			{#if selectAllActions}
				{@render selectAllActions()}
			{/if}
		</div>
	{/if}

	{#if items.length > 0}
		<!-- Items list -->
		<div class="overflow-y-auto">
			<div class="border rounded-md bg-surface-tertiary">
				{#each items as item (item.key)}
					{@const isSelectable = selectablePredicate(item)}
					{@const isSelected = selectedItems.includes(item.key)}
					{@const status = deploymentStatus[item.key]}
					{@const isDeployed = status?.status === 'deployed'}
					{@const blockedReason =
						!isSelectable && !isDeployed ? selectBlockedReason?.(item) : undefined}

					<Row
						isSelectable={!hideSelection && isSelectable && !isDeployed}
						selectDisabledReason={blockedReason}
						selectOnRowClick={!hideSelection}
						alignWithSelectable={!hideSelection}
						disabled={!hideSelection && (blockedReason ? false : !isSelectable)}
						selected={isSelected && !isDeployed}
						onSelect={() => handleSelect(item)}
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
						{#snippet pathDisplay()}
							{#if itemPath}
								{@render itemPath(item)}
							{:else}
								{item.kind !== 'resource' &&
								item.kind !== 'variable' &&
								item.kind !== 'resource_type'
									? item.path
									: ''}
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
			<div class="text-hint text-xs">{emptyMessage}</div>
		</div>
	{/if}

	<!-- Footer -->
	{#if footer}
		<div class="pt-4">
			{@render footer()}
		</div>
	{/if}
</div>
