<script lang="ts">
	import Fuse from 'fuse.js'
	import IconedResourceType from './IconedResourceType.svelte'

	import Modal from './Modal.svelte'

	type Item = Record<string, any>
	export let pickCallback: (path: string, f: string) => void
	export let loadItems: () => Promise<Item[] | undefined>
	export let extraField: string
	export let itemName: string
	export let closeOnClick = true
	export let noItemMessage = ''

	let items: Item[] | undefined = []
	let filteredItems: Item[] | undefined = []
	let itemsFilter = ''

	const fuseOptions = {
		includeScore: false,
		keys: ['path', extraField]
	}
	const fuse: Fuse<Item> = new Fuse(items, fuseOptions)

	export function openModal() {
		loadItems().then((v) => {
			items = v
			if (items) {
				fuse.setCollection(items)
			}
		})
		modal.openModal()
	}

	$: filteredItems =
		itemsFilter.length > 0 && items ? fuse.search(itemsFilter).map((value) => value.item) : items

	let modal: Modal
</script>

<Modal bind:this={modal} z="z-30">
	<div slot="title">Search a {itemName}</div>
	<div slot="content">
		{#if filteredItems}
			<div class="w-12/12 pb-4">
				<input placeholder="Search {itemName}" bind:value={itemsFilter} class="search-item" />
			</div>
			<ul class="divide-y divide-gray-200">
				{#each filteredItems as obj}
					<li
						class="py-4 px-1 gap-1 flex flex-col hover:bg-white hover:border text-black cursor-pointer"
						on:click={() => {
							if (closeOnClick) {
								modal.closeModal()
							}
							pickCallback(obj['path'], obj[extraField])
						}}
					>
						<p class="text-sm font-semibold flex flex-row">
							{#if `app` in obj}
								<IconedResourceType silent={true} name={obj['app']} />
								<span class="mr-2" />
							{/if}
							<span class="mr-2">{obj[extraField]}</span><span class="font-normal break-all"
								>{obj['path'] ?? ''}</span
							>
						</p>
						<p class="text-xs italic">{obj['description'] ?? ''}</p>
					</li>
				{/each}
			</ul>
		{:else}
			<span class="mt-2 text-sm text-red-400">{noItemMessage}</span>
		{/if}
	</div>
	<span slot="submission">
		<slot name="submission" />
	</span>
</Modal>
