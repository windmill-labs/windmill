<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import { IndexSearchService } from '$lib/gen'
	import { clickOutside, displayDateOnly } from '$lib/utils'
	import TimeAgo from './TimeAgo.svelte'
	import {
		ChevronDown,
		DollarSignIcon,
		HomeIcon,
		PlayIcon,
		Search,
		SearchCodeIcon,
		SearchIcon
	} from 'lucide-svelte'
	import JobPreview from './runs/JobPreview.svelte'
	import { Tab, Tabs } from './common'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from './ContentSearchInner.svelte'
	import { goto } from '$app/navigation'
	import QuickMenuItem from './search/QuickMenuItem.svelte'
	import { workspaceStore } from '$lib/stores'
	import uFuzzy from '@leeoniya/ufuzzy'
	import DropdownV2 from './DropdownV2.svelte'

	export let open: boolean = false

	let searchTerm: string = ''
	let textInput: HTMLInputElement
	let results: any[] = []
	let selectedId: string | undefined = undefined
	let selectedWorkspace: string | undefined = undefined
	let contentSearch: ContentSearchInner | undefined = undefined

	type SearchMode = 'default' | 'switch-mode' | 'runs' | 'content' | 'logs'

	let tab: SearchMode = 'default'

	type quickMenuItem = { id: string; label: string; action: () => void; icon?: any }
	let switchModeItems: quickMenuItem[] = [
		{
			id: 'run-search',
			label: 'Search across runs',
			action: () => switchMode('runs'),
			icon: SearchIcon
		},
		{
			id: 'content-search',
			label: 'Search scripts/flows/apps based on content',
			action: () => switchMode('content'),
			icon: SearchCodeIcon
		},
		{
			id: 'log-search',
			label: 'Search windmill logs',
			action: () => switchMode('logs'),
			icon: SearchIcon
		}
	]
	let defaultMenuItems: quickMenuItem[] = [
		{ id: 'home', label: 'Go to Home', action: () => gotoPage('/'), icon: HomeIcon },
		{ id: 'runs', label: 'Go to Runs', action: () => gotoPage('/runs'), icon: PlayIcon },
		{
			id: 'variables',
			label: 'Go to Variables',
			action: () => gotoPage('/variables'),
			icon: DollarSignIcon
		},
		{ id: 'resources', label: 'Go to Resources', action: () => gotoPage('/resources') },
		{ id: 'schedules', label: 'Go to Schedules', action: () => gotoPage('/schedules') },
		...switchModeItems
	]

	let itemMap = {
		default: defaultMenuItems,
		'switch-mode': switchModeItems,
		runs: [] as any[],
		content: [] as any[],
		logs: [] as any[]
	}

	$: tab === 'content' && contentSearch?.open()

	$: tab && switchPrompt(tab)

	function switchPrompt(tab: string) {
		if (tab === 'default') {
			searchTerm = ''
		}
		if (tab === 'runs') {
			searchTerm = '>'
		}
		if (tab === 'content') {
			searchTerm = '>'
		}
		if (tab === 'switch-mode') {
			searchTerm = '?'
		}
		if (tab === 'switch-mode') {
			searchTerm = '>'
		}
		selectedItem = selectItem(0)
	}

	function removePrefix(str: string, prefix: string): string {
		if (str.startsWith(prefix)) {
			return str.substring(prefix.length)
		}
		return str
	}

	let opts: uFuzzy.Options = {}

	let uf = new uFuzzy(opts)
	let defaultMenuItemLabels = defaultMenuItems.map((item) => item.label)
	let switchModeItemLabels = switchModeItems.map((item) => item.label)

	function fuzzyFilter(filter: string, items: any[], itemsPlainText: string[]) {
		if (filter === '') {
			return items
		}
		let idxs = uf.filter(itemsPlainText, filter) ?? []

		let info = uf.info(idxs, itemsPlainText, filter)
		let order = uf.sort(info, itemsPlainText, filter)

		let r: any[] = []
		for (let o of order) {
			r.push(items[info.idx[o]])
		}
		return r
	}
	async function handleSearch() {
		if (searchTerm === '') {
			tab = 'default'
		}
		if (searchTerm === '?') {
			tab = 'switch-mode'
		}

		if (tab === 'default') {
			itemMap['default'] = fuzzyFilter(searchTerm, defaultMenuItems, defaultMenuItemLabels)
		}
		if (tab === 'switch-mode') {
			itemMap['switch-mode'] = fuzzyFilter(
				removePrefix(searchTerm, '?'),
				switchModeItems,
				switchModeItemLabels
			)
		}
		if (tab === 'runs') {
			// only search if hasn't been called in some small time. Show load animation while wating. Also add a cache that resets everytime the modal is closed
			const s = removePrefix(searchTerm, '>')
			try {
				itemMap['runs'] = await IndexSearchService.searchJobsIndex({ query: s })
			} catch (error) {
				itemMap['runs'] = []
				throw error
			}
		}
		selectedItem = selectItem(0)
	}

	function selectItem(index: number) {
		if (!itemMap[tab] || itemMap[tab].length <= index) {
			return undefined
		}
		return itemMap[tab][index]
	}

	let selectedItem: any
	async function handleKeydown(event: KeyboardEvent) {
		if (event.ctrlKey && event.key === 'k') {
			event.preventDefault()
			open = !open
			await tick()
			if (open) {
				selectedItem = selectItem(0)
				textInput.focus()
				textInput.select()
			}
		}
		if (open) {
			if (event.key === 'Escape') {
				event.preventDefault()
				if (searchTerm.length != 0 || tab != 'default') {
					searchTerm = ''
					tab = 'default'
					textInput?.focus()
				} else {
					open = false
				}
			}
			if (event.key === 'ArrowDown') {
				event.preventDefault()
				let idx = itemMap[tab].indexOf(selectedItem)
				if (idx != -1) {
					idx = (idx + 1) % itemMap[tab].length
					selectedItem = selectItem(idx)
				}
			} else if (event.key === 'ArrowUp') {
				event.preventDefault()
				let idx = itemMap[tab].indexOf(selectedItem)
				if (idx != -1) {
					idx = (idx - 1 + itemMap[tab].length) % itemMap[tab].length
					selectedItem = selectItem(idx)
				}
			}
		}
	}

	function switchMode(mode: SearchMode) {
		tab = mode
	}

	function gotoPage(path: string) {
		open = false
		searchTerm = ''
		goto(path)
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})
	$: searchTerm, handleSearch()

	function placeholderForMode(mode: SearchMode): string | null | undefined {
		switch (mode) {
		}
		switch (mode) {
			case 'default':
				return 'Search or type `?` for search options'
			case 'runs':
				return 'Search across all jobs'
			default:
				return 'Search...'
		}
	}

</script>

{#if open}
	<Portal>
		<div
			class={twMerge(
				`fixed top-0 bottom-0 left-0 right-0 transition-all duration-50`,
				' bg-black bg-opacity-60',
				'z-[1100]'
			)}
		>
			<div
				class={'max-w-4xl max-h-[80vh] min-h-[20vh] lg:mx-auto mx-10 mt-40 bg-surface rounded-lg relative'}
				use:clickOutside={false}
				on:click_outside={() => {
					// open = false
				}}
			>
				<div class="px-4 py-2 border-b items-center">
					<div class="quick-search-input flex flex-row gap-1 items-center mb-2">
						<Search />
						<input
							bind:this={textInput}
							type="text"
							class="quick-search-input"
							bind:value={searchTerm}
							placeholder={placeholderForMode(tab)}
						/>
					</div>
					{#if tab === 'default' || tab === 'switch-mode'}
						{#each itemMap[tab] ?? [] as el}
							<QuickMenuItem
								on:select={el?.action}
								on:hover={() => (selectedItem = el)}
								id={el?.id}
								hovered={el?.id === selectedItem?.id}
								label={el?.label}
								icon={el?.icon}
							/>
						{/each}
					{/if}
					{#if tab !== 'switch-mode'}
						<div
							class="flex flex-row items-center text-xs font-bold font-small-caps border-b-2 border-gray-300 p-1 mt-2"
						>
							<span class="h-6 px-1"> Search </span>
							<span>
								<DropdownV2
									items={[
										{
											displayName: '> completed runs',
											action: () => (tab = 'runs')
										},
										{
											displayName: '! service logs',
											action: () => (tab = 'logs')
										},
										{
											displayName: '# flows/apps/scripts by content',
											action: () => (tab = 'content')
										},
										{
											displayName: 'flows/apps/scripts by name',
											action: () => (tab = 'default')
										}
									]}
								>
									<svelte:fragment slot="buttonReplacement">
										<div class="flex flex-row text-xs items-center">
											<div
												class="pl-1 h-6 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
											>
												<span class="">{tab}</span>
												<ChevronDown class="w-5 h-5" />
											</div>
										</div>
									</svelte:fragment>
								</DropdownV2>
							</span>
						</div>
					{/if}
					{#if tab === 'content'}
						<ContentSearchInner search={removePrefix(searchTerm, '>')} bind:this={contentSearch} />
					{:else if tab === 'logs'}
						Not implemented yet
					{:else if tab === 'runs'}
						<div class="flex h-96">
							<!-- {#if !results || results.length == 0} -->
							<!-- 	No matches found -->
							<!-- {:else} -->
							<div class="w-3/12 overflow-scroll">
								{#each itemMap[tab] ?? ['a'] as r}
									<QuickMenuItem
										on:hover={() => {
											selectedItem = r
											selectedId = r?.document.id[0]
											selectedWorkspace = r?.document.workspace_id[0]
										}}
										on:select={() => {
											open = false
											goto(`/run/${r?.document.id[0]}`)
										}}
										id={r?.document.id[0]}
										hovered={selectedItem && r?.document.id[0] === selectedItem.document.id[0]}
										icon={r?.icon}
									>
										<svelte:fragment slot="itemReplacement">
											<button
												class={twMerge(
													`w-full flex items-center justify-between gap-4 py-2 px-4 text-left border rounded-sm transition-a`,
													r.document.id === selectedItem?.document?.id ? 'bg-surface-hover' : ''
												)}
												on:click={() => {
													selectedId = r.document.id[0]
													selectedWorkspace = r.document.workspace_id[0]
												}}
											>
												<div
													class="w-full h-full items-center text-xs font-normal grid grid-cols-8 gap-4 min-w-0"
												>
													<div class="">
														<div
															class="rounded-full w-2 h-2 {r.document.success[0]
																? 'bg-green-400'
																: 'bg-red-400'}"
														/>
													</div>
													<div class="col-span-2">
														{r.document.script_path}
													</div>
													<div
														class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
													>
														{displayDateOnly(new Date(r.document.created_at[0]))}
													</div>
													<div
														class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
													>
														<TimeAgo date={r.document.created_at[0] ?? ''} />
													</div>
													<!-- <div class="col-span-2"> -->
													<!-- 	<a -->
													<!-- 		target="_blank" -->
													<!-- 		href="/run/{`asdasd`}?workspace={$workspaceStore}" -->
													<!-- 		class="text-right float-right text-secondary" -->
													<!-- 		title="See run detail in a new tab" -->
													<!-- 	> -->
													<!-- 		<ExternalLink size={16} /> -->
													<!-- 	</a> -->
													<!-- </div> -->
												</div>
											</button>
											<!-- <div class="p-2 mb-2 cursor-pointer hover:bg-gray-200 rounded"> -->
											<!-- </div> -->
										</svelte:fragment>
									</QuickMenuItem>
								{/each}
							</div>
							{#if selectedItem == undefined}
								select a result to preview
							{:else}
								<div class="w-9/12 overflow-y-scroll">
									<JobPreview id={selectedItem.document.id[0]} workspace={selectedWorkspace} />
								</div>
							{/if}
							<!-- {/if} -->
						</div>
					{/if}
				</div>
			</div>
		</div>
	</Portal>
{/if}

<style>
	.quick-search-input {
		outline: none;
		border: none !important;
		box-shadow: none !important;
	}

	.quick-search-input:focus-visible {
		outline: none !important;
	}
</style>
