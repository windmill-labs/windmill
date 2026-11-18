<script lang="ts">
	import { Folder, User } from 'lucide-svelte'
	import LabelBadge from '../common/badge/LabelBadge.svelte'
	import type { BadgeColor, BadgeIconProps } from '../common/badge/model'
	import { appIconComponent } from '../icons/appIcon.svelte'
	import { integrationDisplayName } from '../resourceTypeDisplay'
	import { untrack } from 'svelte'
	import { useHostedPage } from '../hostedPage'

	interface Props {
		filters: string[]
		selectedFilter?: string | undefined
		resourceType?: boolean
		queryName?: string
		syncQuery?: boolean
		bottomMargin?: boolean
		// Chip look. `icon` is drawn before every chip; unset, the icon follows the value
		// (user/folder prefix, or the app icon under `resourceType`).
		color?: BadgeColor
		icon?: BadgeIconProps['icon']
		// Emit the chips as flex items of the parent instead of a row of their own, so several
		// ListFilters can share one line. The parent owns layout and spacing, so
		// `bottomMargin` does not apply.
		inline?: boolean
	}

	let {
		filters,
		selectedFilter = $bindable(undefined),
		resourceType = false,
		queryName = 'filter',
		syncQuery = false,
		bottomMargin = true,
		color = 'transparent',
		icon,
		inline = false
	}: Props = $props()

	const hosted = useHostedPage()

	function loadFilterFromUrl() {
		const search = hosted ? hosted.search : window.location.search
		selectedFilter = new URLSearchParams(search).get(queryName) ?? undefined
	}

	function writeQuery(value: string | undefined) {
		if (!syncQuery) return
		const params = new URLSearchParams(hosted ? hosted.search : window.location.search)
		if (value != undefined) params.set(queryName, value)
		else params.delete(queryName)
		if (hosted) hosted.setSearch(params.size ? `?${params}` : '')
		else window.history.pushState(history.state, '', `?${params}`)
	}

	$effect(() => {
		if (!syncQuery) return
		if (hosted) {
			hosted.search
			untrack(loadFilterFromUrl)
			return
		}
		window.addEventListener('popstate', loadFilterFromUrl)
		return () => window.removeEventListener('popstate', loadFilterFromUrl)
	})

	untrack(loadFilterFromUrl)

	let filtersAndSelected = $derived(
		selectedFilter
			? filters.includes(selectedFilter)
				? filters
				: [selectedFilter, ...filters]
			: filters
	)
</script>

{#if Array.isArray(filtersAndSelected) && filtersAndSelected.length > 0}
	<div
		class={inline ? 'contents' : `gap-2 w-full flex flex-wrap ${bottomMargin ? 'my-4' : 'mt-4'}`}
	>
		{#each filtersAndSelected as filter (filter)}
			<div data-chip data-selected={filter === selectedFilter ? '' : undefined}>
				<LabelBadge
					{color}
					icon={icon ??
						(resourceType
							? undefined
							: filter.startsWith('u/')
								? User
								: filter.startsWith('f/')
									? Folder
									: undefined)}
					selected={filter === selectedFilter}
					onclick={() => {
						selectedFilter = selectedFilter == filter ? undefined : filter
						writeQuery(selectedFilter)
					}}
				>
					{#if !icon && resourceType}
						{@const AppIcon = appIconComponent(filter)}
						{#if AppIcon}
							<AppIcon height="12px" width="12px" />
						{/if}
					{/if}
					{resourceType ? integrationDisplayName(filter) : filter}
				</LabelBadge>
			</div>
		{/each}
	</div>
{/if}
