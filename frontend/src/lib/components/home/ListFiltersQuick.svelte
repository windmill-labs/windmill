<script lang="ts">
	import { Folder, User, Circle } from 'lucide-svelte'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { onDestroy, onMount } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let filters: string[]
	export let selectedFilter:
		| { kind: 'inline' | 'owner' | 'integrations'; name: string | undefined }
		| undefined = undefined
	$: selectedAppFilter = selectedFilter?.kind === 'integrations' ? selectedFilter?.name : undefined

	export let resourceType = false
	export let queryName = 'filter'
	export let syncQuery = false

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
		selectedFilter = queryValue ? { kind: 'integrations', name: queryValue } : undefined
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

	$: filtersAndSelected = selectedAppFilter
		? filters.includes(selectedAppFilter)
			? filters
			: [selectedAppFilter, ...filters]
		: filters

	let icon: any
</script>

{#if Array.isArray(filtersAndSelected) && filtersAndSelected.length > 0}
	{#each filtersAndSelected as filter (filter)}
		<div>
			<button
				class={twMerge(
					'w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
					filter === selectedAppFilter ? 'bg-surface-hover' : ''
				)}
				on:click={() => {
					selectedFilter =
						selectedAppFilter == filter ? undefined : { kind: 'integrations', name: filter }
					if (selectedFilter) {
						setQuery(new URL(window.location.href), queryName, selectedAppFilter)
					} else {
						setQuery(new URL(window.location.href), queryName, undefined)
					}
				}}
			>
				<div class="flex justify-center flex-row items-center gap-2">
					{#if resourceType}
						{#if (icon = getIconComponent(filter))}
							<svelte:component this={icon} height="14px" width="14px" />
						{:else}
							<div
								class="w-[14px] h-[14px] text-gray-400 flex flex-row items-center justify-center"
							>
								<Circle size="12" />
							</div>
						{/if}
					{:else if filter.startsWith('u/')}
						<User class="mr-0.5" size={14} />
					{:else if filter.startsWith('f/')}
						<Folder class="mr-0.5" size={14} />
					{/if}
					<span class="text-left text-2xs text-primary font-normal">{filter}</span>
				</div>
			</button>
		</div>
	{/each}
{/if}
