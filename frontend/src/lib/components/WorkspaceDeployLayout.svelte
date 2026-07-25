<script lang="ts">
	import { Folder, Loader2, User } from 'lucide-svelte'
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

	interface DeployGroup {
		/** `f/<folder>` or `u/<user>`; '' for items outside both conventions. */
		key: string
		label: string
		groupKind: 'folder' | 'user' | 'other'
		items: DeployableItem[]
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
		/** Rendered on the right of each folder/user group header — receives the
		 * group's items so pages can roll up their own badges (e.g. conflicts). */
		groupActions?: Snippet<[DeployableItem[]]>
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
		groupActions,
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

	function groupOf(path: string): Pick<DeployGroup, 'key' | 'label' | 'groupKind'> {
		const parts = path.split('/')
		// >= 2 so a folder item itself (path `f/<name>`, kind 'folder') lands in
		// its folder's group next to the items that need it, not in "other".
		if (parts.length >= 2 && (parts[0] === 'f' || parts[0] === 'u')) {
			const key = `${parts[0]}/${parts[1]}`
			return { key, label: key, groupKind: parts[0] === 'f' ? 'folder' : 'user' }
		}
		return { key: '', label: 'other', groupKind: 'other' }
	}

	let groups = $derived.by((): DeployGroup[] => {
		const byKey = new Map<string, DeployGroup>()
		for (const item of items) {
			const g = groupOf(item.path)
			let group = byKey.get(g.key)
			if (!group) {
				group = { ...g, items: [] }
				byKey.set(g.key, group)
			}
			group.items.push(item)
		}
		const rank = { folder: 0, user: 1, other: 2 }
		return [...byKey.values()].sort(
			(a, b) => rank[a.groupKind] - rank[b.groupKind] || a.key.localeCompare(b.key)
		)
	})

	// A lone catch-all group would just duplicate "Select all" — skip headers then.
	let showGroupHeaders = $derived(groups.some((g) => g.groupKind !== 'other'))

	// Mirrors the per-row checkbox condition: already-deployed rows keep their
	// selection frozen, so they don't count toward the group's tri-state.
	function groupSelectable(group: DeployGroup): DeployableItem[] {
		return group.items.filter(
			(i) => selectablePredicate(i) && deploymentStatus[i.key]?.status !== 'deployed'
		)
	}

	function toggleGroup(group: DeployGroup) {
		const selectable = groupSelectable(group)
		const target = !selectable.every((i) => selectedItems.includes(i.key))
		// Snapshot which items need flipping before mutating: onToggleItem updates
		// the parent's selection, which feeds back into `selectedItems` mid-loop.
		const toToggle = selectable.filter((i) => selectedItems.includes(i.key) !== target)
		for (const item of toToggle) {
			onToggleItem?.(item)
		}
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
				{#each groups as group (group.key)}
					{#if showGroupHeaders}
						{@const selectable = groupSelectable(group)}
						{@const selectedCount = selectable.filter((i) => selectedItems.includes(i.key)).length}
						<!-- The disabled-state hint lives on the row: a disabled Checkbox is
						     pointer-events-none, so a title on the input would never show. -->
						<div
							class="sticky top-0 z-10 flex items-center gap-2 px-4 py-1.5 bg-surface-secondary border-b first:rounded-t-md"
							title={selectable.length === 0 ? 'No selectable items in this group' : undefined}
						>
							{#if !hideSelection}
								<Checkbox
									checked={selectable.length > 0 && selectedCount === selectable.length}
									indeterminate={selectedCount > 0 && selectedCount < selectable.length}
									disabled={selectable.length === 0}
									title={selectedCount === selectable.length
										? `Deselect all in ${group.label}`
										: `Select all in ${group.label}`}
									onChange={() => toggleGroup(group)}
								/>
							{/if}
							{#if group.groupKind === 'folder'}
								<Folder size={14} class="text-tertiary shrink-0" />
							{:else if group.groupKind === 'user'}
								<User size={14} class="text-tertiary shrink-0" />
							{/if}
							<span class="text-xs font-semibold text-secondary truncate">{group.label}</span>
							<span class="text-2xs text-tertiary whitespace-nowrap">
								{group.items.length} item{group.items.length !== 1 ? 's' : ''}{selectedCount > 0
									? ` · ${selectedCount} selected`
									: ''}
							</span>
							{#if groupActions}
								<div class="ml-auto flex items-center gap-1">
									{@render groupActions(group.items)}
								</div>
							{/if}
						</div>
					{/if}
					{#each group.items as item (item.key)}
						{@const isSelectable = selectablePredicate(item)}
						{@const isSelected = selectedItems.includes(item.key)}
						{@const status = deploymentStatus[item.key]}
						{@const isDeployed = status?.status === 'deployed'}
						{@const blockedReason =
							!isSelectable && !isDeployed ? selectBlockedReason?.(item) : undefined}
						{@const showPath =
							item.kind !== 'resource' && item.kind !== 'variable' && item.kind !== 'resource_type'}
						<!-- The group header already names the folder — the path line only
						     carries what's below it. -->
						{@const subPath =
							group.key && item.path.startsWith(group.key + '/')
								? item.path.slice(group.key.length + 1)
								: item.path}

						<Row
							isSelectable={!hideSelection && isSelectable && !isDeployed}
							selectDisabledReason={blockedReason}
							selectOnRowClick={!hideSelection}
							alignWithSelectable={!hideSelection}
							disabled={!hideSelection && (blockedReason ? false : !isSelectable)}
							selected={isSelected && !isDeployed}
							onSelect={() => handleSelect(item)}
							path={showPath ? item.path : ''}
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
									{showPath ? (showGroupHeaders ? subPath : item.path) : ''}
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
