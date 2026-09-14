<script lang="ts">
	import { Folder, User } from 'lucide-svelte'
	import { Badge } from '../common'
	import type { BadgeColor, BadgeIconProps } from '../common/badge/model'
	import { appIconComponent } from '../icons'
	import { onDestroy, onMount } from 'svelte'

	interface Props {
		filters: string[]
		selectedFilter?: string | undefined
		resourceType?: boolean
		queryName?: string
		syncQuery?: boolean
		bottomMargin?: boolean
		// Keep only the first N filters visible, the rest behind a "…" toggle. Unset
		// shows every one.
		maxDisplayed?: number
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
		maxDisplayed,
		color = 'transparent',
		icon,
		inline = false
	}: Props = $props()

	const queryChange: (value: URL) => void = (url: URL) => {
		if (syncQuery) {
			window.history.pushState(history.state, '', `?${url?.searchParams.toString()}`)
		}
	}

	const eventListener = (e: PopStateEvent) => {
		if (syncQuery) {
			loadFilterFromUrl()
		}
	}

	onMount(() => {
		window.addEventListener('popstate', eventListener)
	})

	onDestroy(() => {
		window.removeEventListener('popstate', (e) => eventListener(e))
	})

	loadFilterFromUrl()

	function loadFilterFromUrl() {
		let queryValue = new URL(window.location.href).searchParams.get(queryName) ?? undefined
		selectedFilter = queryValue
	}

	export async function setQuery(url: URL, key: string, value: string | undefined): Promise<void> {
		if (value != undefined) {
			url.searchParams.set(key, value)
		} else {
			url.searchParams.delete(key)
		}
		queryChange(url)
	}

	let filtersAndSelected = $derived(
		selectedFilter
			? filters.includes(selectedFilter)
				? filters
				: [selectedFilter, ...filters]
			: filters
	)

	let expanded = $state(false)
	let truncated = $derived.by(() => {
		if (maxDisplayed == undefined || filtersAndSelected.length <= maxDisplayed)
			return filtersAndSelected
		const shown = filtersAndSelected.slice(0, maxDisplayed)
		// Clicking the selected chip is how the filter is cleared, so it is never truncated
		// away, however far down the order it sits.
		if (selectedFilter && !shown.includes(selectedFilter)) shown.push(selectedFilter)
		return shown
	})
	// Derived from what is actually withheld, so the selected chip kept above never counts
	// as hidden — otherwise the toggle offers to reveal a chip that is already on screen.
	let hiddenCount = $derived(filtersAndSelected.length - truncated.length)
	let displayedFilters = $derived(expanded ? filtersAndSelected : truncated)
</script>

{#if Array.isArray(filtersAndSelected) && filtersAndSelected.length > 0}
	<div
		class={inline ? 'contents' : `gap-2 w-full flex flex-wrap ${bottomMargin ? 'my-4' : 'mt-4'}`}
	>
		{#each displayedFilters as filter (filter)}
			<div>
				<Badge
					class="inline-flex items-center gap-1 align-middle"
					onclick={() => {
						selectedFilter = selectedFilter == filter ? undefined : filter
						if (selectedFilter) {
							setQuery(new URL(window.location.href), queryName, selectedFilter)
						} else {
							setQuery(new URL(window.location.href), queryName, undefined)
						}
					}}
					{color}
					clickable
					selected={filter === selectedFilter}
				>
					<span style="height: 12px" class="-mt-0.5">
						{#if icon}
							{@const Icon = icon}
							<Icon class="mr-0.5" size={12} />
						{:else if resourceType}
							{@const SvelteComponent = appIconComponent(filter)}
							<SvelteComponent height="14px" width="14px" />
						{:else if filter.startsWith('u/')}
							<User class="mr-0.5" size={14} />
						{:else if filter.startsWith('f/')}
							<Folder class="mr-0.5" size={14} />
						{/if}
					</span>
					{filter}
					{#if filter === selectedFilter}&cross;{/if}
				</Badge>
			</div>
		{/each}
		{#if hiddenCount > 0}
			<div>
				<Badge
					class="inline-flex items-center gap-1 align-middle"
					{color}
					clickable
					title={expanded ? 'Show fewer' : `Show ${hiddenCount} more`}
					onclick={() => (expanded = !expanded)}
				>
					{expanded ? 'Show less' : '…'}
				</Badge>
			</div>
		{/if}
	</div>
{/if}
