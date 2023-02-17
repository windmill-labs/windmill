<script lang="ts">
	import type { AppInput, StaticInput } from '../../../inputType'
	import { Loader2 } from 'lucide-svelte'
	import { Popup } from '../../../../common'
	import { fade } from 'svelte/transition'
	import Popover from '../../../../Popover.svelte'

	export let componentInput: StaticInput<string> & Extract<AppInput, { fieldType: 'icon-select' }>
	let anchor: HTMLElement
	let loading = false
	let items: { label: string, icon: any }[]
	let filteredItems: { label: string, icon: any }[]
	let search = ''

	$: if(search) {
		filteredItems = items.filter(item => {
			return item.label.toLowerCase().includes(search.toLowerCase())
		})
	} else {
		filteredItems = items
	}

	async function getData() {
		if(items) return;
		loading = true
		const data = await import('lucide-svelte/dist/svelte/icons')
		filteredItems = items = Object.entries(data)
			.filter(([key]) => !(key.endsWith('Icon') || key.startsWith('Lucide')))
			.map(([key, icon]) => ({ label: key, icon }))
		loading = false
	}

	function formatName(name?: string) {
		return name?.replace(/([A-Z])/g, ' $1').trim() || ''
	}
</script>

<input
	readonly
	value={formatName(componentInput.value)}
	bind:this={anchor}
	on:click={getData}
/>
{#if anchor}
	<Popup ref={anchor} options={{ placement: 'bottom' }} transition={fade}>
		<div class="max-w-xs shadow-[0_10px_40px_-5px_rgba(0,0,0,0.25)] bg-white rounded-md p-2">
			{#if loading}
				<div class="center-center p-2">
					<Loader2 class="animate-spin" size={18} />
				</div>
			{:else}
				{#if filteredItems}
					<input bind:value={search} type="text" placeholder="Search" class="col-span-4 mb-2">
					<div class="grid gap-1 grid-cols-4 max-h-[300px] overflow-auto">
						{#each filteredItems as {label, icon}}
							{@const formatedLabel = formatName(label)}
							<button
								type="button"
								title={formatedLabel}
								on:click={() => {
									componentInput.value = label
									anchor.focus()
								}}
								transition:fade|local={{ duration: 200 }}
								class="w-full center-center flex-col font-normal p-1 
								hover:bg-gray-100 focus:bg-gray-100 rounded duration-200 
								{label === componentInput.value ? 'text-blue-600 bg-blue-50 pointer-events-none' : ''}"
							>
								<svelte:component this={icon} size={22} />
								<span class="inline-block w-full text-[10px] ellipsize pt-0.5">
									{formatedLabel}
								</span>
							</button>
						{/each}
					</div>
				{:else}
					<div class="text-center text-sm text-gray-600 p-2">
						Couldn't load options
					</div>
				{/if}
			{/if}
		</div>
	</Popup>
{/if}