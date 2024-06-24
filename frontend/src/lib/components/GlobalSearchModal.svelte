<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import Modal from './common/modal/Modal.svelte'
	import { IndexSearchService } from '$lib/gen'
	import { classNames, displayDateOnly } from '$lib/utils'
	import TimeAgo from './TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import JobPreview from './runs/JobPreview.svelte'

	export let open: boolean = false

	let searchTerm: string = ''
	let textInput: HTMLInputElement
	let results: any = undefined
	let selectedId: string | undefined = undefined
	let selectedWorkspace: string | undefined = undefined

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

	const handleKeydown = async (event) => {
		if (event.ctrlKey && event.key === 'k') {
			event.preventDefault()
			open = !open
			await tick()
			if (open) {
				textInput.focus()
				textInput.select()
			}
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
</script>

<Modal title="Global search" class="sm:max-w-none sm:my-50 sm:h-1/2" bind:open>
	<input bind:this={textInput} type="text" bind:value={searchTerm} placeholder="Search..." />
	<div class="flex">
		{#if !results || results.length == 0}
			No matches found
		{:else}
			<div class="w-3/12">
				{#each results as r}
					<button
						class={classNames(
							`w-full flex items-center justify-between gap-4 py-2 px-4 text-left border rounded-sm hover:bg-surface-hover transition-a`
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
</Modal>
