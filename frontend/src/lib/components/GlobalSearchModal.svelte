<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import Modal from './common/modal/Modal.svelte'
	import { IndexSearchService } from '$lib/gen'

	export let open: boolean = false
	let searchTerm: string = ''
	let textInput: HTMLInputElement
	let results: any = undefined

	async function handleSearch() {
		// only search if hasn't been called in some small time. Show load animation while wating. Also add a cache that resets everytime the modal is closed
		try {
			results = await IndexSearchService.searchJobsIndex({ query: searchTerm })
		} catch (error) {
			results = []
			throw error
		}
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

<Modal title="Search jobs" class="sm:max-w-none sm:my-50" bind:open>
	<input
		bind:this={textInput}
		type="text"
		bind:value={searchTerm}
		placeholder="Search..."
	/>
	<div>
		{#each results as r}
			<div>
			{r}
			</div>
		{/each}
	</div>
</Modal>
