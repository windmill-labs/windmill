<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Folder, User } from 'lucide-svelte'
	import { Badge } from '../common'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { onDestroy, onMount } from 'svelte'

	interface Props {
		filters: string[]
		selectedFilter?: string | undefined
		resourceType?: boolean
		queryName?: string
		syncQuery?: boolean
		bottomMargin?: boolean
	}

	let {
		filters,
		selectedFilter = $bindable(undefined),
		resourceType = false,
		queryName = 'filter',
		syncQuery = false,
		bottomMargin = true
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

	function getIconComponent(name: string) {
		return APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
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
</script>

{#if Array.isArray(filtersAndSelected) && filtersAndSelected.length > 0}
	<div class={`gap-2 w-full flex flex-wrap ${bottomMargin ? 'my-4' : 'mt-4'}`}>
		{#each filtersAndSelected as filter (filter)}
			<div>
				<Badge
					class={classNames(
						'cursor-pointer inline-flex items-center gap-1 align-middle',
						filter === selectedFilter ? 'hover:bg-blue-200' : 'hover:bg-gray-200'
					)}
					on:click={() => {
						selectedFilter = selectedFilter == filter ? undefined : filter
						if (selectedFilter) {
							setQuery(new URL(window.location.href), queryName, selectedFilter)
						} else {
							setQuery(new URL(window.location.href), queryName, undefined)
						}
					}}
					color={filter === selectedFilter ? 'blue' : 'gray'}
					baseClass={filter === selectedFilter ? 'border border-blue-500' : 'border'}
				>
					<span style="height: 12px" class="-mt-0.5">
						{#if resourceType}
							{@const SvelteComponent = getIconComponent(filter)}
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
	</div>
{/if}
