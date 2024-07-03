<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import Modal from './common/modal/Modal.svelte'
	import { IndexSearchService } from '$lib/gen'
	import { classNames, clickOutside, displayDateOnly } from '$lib/utils'
	import TimeAgo from './TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import JobPreview from './runs/JobPreview.svelte'
	import { Tab, Tabs } from './common'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from './ContentSearchInner.svelte'
	import { goto } from '$app/navigation'

	export let open: boolean = false

	let searchTerm: string = ''
	let textInput: HTMLInputElement
	let results: any[] = []
	let selectedId: string | undefined = undefined
	let selectedWorkspace: string | undefined = undefined
	let contentSearch: ContentSearchInner | undefined = undefined

	$: tab === 'content-search' && contentSearch?.open()

	interface menuItem {
		id: string
	}

	let items: any[] = []

	async function handleSearch() {
		// only search if hasn't been called in some small time. Show load animation while wating. Also add a cache that resets everytime the modal is closed
		try {
			results = await IndexSearchService.searchJobsIndex({ query: searchTerm })
		} catch (error) {
			results = []
			throw error
		}
		console.log(results)
	}

	let activeIndex = 0

	const handleKeydown = async (event) => {
		if (event.ctrlKey && event.key === 'k') {
			event.preventDefault()
			open = !open
			await tick()
			if (open) {
				activeIndex = 0
				textInput.focus()
				textInput.select()
			}
		}
		if (open) {
			if (event.key === 'Escape') {
				event.preventDefault()
				if (searchTerm.length != 0 || tab != 'quick') {
					searchTerm = ''
					tab = 'quick'
					textInput?.focus()
				} else {
					open = false
				}
			}
			if (event.key === 'ArrowDown') {
				event.preventDefault()
				activeIndex = (activeIndex + 1) % items.length
			} else if (event.key === 'ArrowUp') {
				event.preventDefault()
				activeIndex = (activeIndex - 1 + items.length) % items.length
			}
		}
	}

	$: selectedItem = items.length > 0 ? items[activeIndex] : undefined
	$: selectedItem && console.log(selectedItem)
	$: tab && results && tabChanged()

	function tabChanged() {

		if (tab === 'quick') {
			items = quickMenu
		}
		else if (tab === 'runs' && results != undefined) {
			items = results
			console.log(items)
		}
	}
	// const closeModal = () => {
	// 	globalSearchModalOpen = false
	// }

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})
	$: searchTerm, handleSearch()

	let tab: 'quick' | 'runs' | 'content-search' | 'logs' = 'quick'

	type quickMenuItem = { id: string; label: string; action: () => void }
	let quickMenu: quickMenuItem[] = [
		{ id: 'home', label: 'Home', action: () => goto('/') },
		{ id: 'runs', label: 'Runs', action: () => goto('/runs') },
		{ id: 'variables', label: 'Variables', action: () => goto('/') },
		{ id: 'resources', label: 'Resources', action: () => goto('/') }
	]
</script>

<!-- <Modal title="Global search" class="sm:max-w-none sm:my-50 sm:h-1/2" bind:open> -->
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
					open = false
				}}
			>
				<div class="px-4 py-2 border-b items-center">
					<div class="w-full overflow-auto scrollbar-hidden">
						<Tabs values={['quick', 'runs', 'content-search', 'logs']} bind:selected={tab}>
							<Tab size="md" value="quick">
								<div class="flex gap-2 items-center my-1"> Quick Access </div>
							</Tab>
							<Tab size="md" value="runs">
								<div class="flex gap-2 items-center my-1"> Runs </div>
							</Tab>
							<Tab size="md" value="content-search">
								<div class="flex gap-2 items-center my-1"> Content Search </div>
							</Tab>
							<Tab size="md" value="logs">
								<div class="flex gap-2 items-center my-1"> Logs </div>
							</Tab>
						</Tabs>
					</div>
					<input
						bind:this={textInput}
						type="text"
						bind:value={searchTerm}
						placeholder="Search..."
					/>
					{#if tab == 'runs'}
						<div class="flex">
							{#if !results || results.length == 0}
								No matches found
							{:else}
								<div class="w-3/12">
									{#each results as r}
										<button
											class={twMerge(
												`w-full flex items-center justify-between gap-4 py-2 px-4 text-left border rounded-sm transition-a`,
												r.document.id === selectedItem?.document?.id ? 'bg-surface-hover' : '',
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
									{/each}
								</div>
								{#if selectedId == undefined}
									select a result to preview
								{:else}
									<div class="w-9/12">
										<JobPreview id={selectedId} workspace={selectedWorkspace} />
									</div>
								{/if}
							{/if}
						</div>
					{:else if tab === 'content-search'}
						<ContentSearchInner search={searchTerm} bind:this={contentSearch} />
					{:else if tab === 'logs'}
						Not implemented yet
					{:else if tab === 'quick'}
						{#each quickMenu as el}
							<div on:click={el.action} class={el.id === selectedItem?.id ? 'bg-surface-hover' : ''}>
								{el.label}
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</div>
	</Portal>
{/if}
<!-- </Modal> -->
