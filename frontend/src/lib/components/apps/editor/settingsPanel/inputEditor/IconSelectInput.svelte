<script lang="ts">
	import type { AppInput, StaticInput } from '../../../inputType'
	import { Loader2, X } from 'lucide-svelte'
	import { Popup } from '../../../../common'
	import { fade } from 'svelte/transition'

	export let componentInput: StaticInput<string> & Extract<AppInput, { fieldType: 'icon-select' }>
	let anchor: HTMLElement
	let loading = false
	let items: { label: string; icon: any }[]
	let filteredItems: { label: string; icon: any }[]
	let search = ''
	let openPopup: () => void

	$: if (search) {
		filteredItems = items.filter((item) => {
			return item.label.toLowerCase().includes(search.toLowerCase())
		})
	} else {
		filteredItems = items
	}

	async function getData() {
		if (items) {
			return openPopup()
		}
		loading = true
		const data = await import('lucide-svelte/dist/svelte/icons')
		filteredItems = items = Object.entries(data)
			.filter(([key]) => !(key.endsWith('Icon') || key.startsWith('Lucide')))
			.map(([key, icon]) => ({ label: key, icon }))
		loading = false
		openPopup()
	}

	function formatName(name?: string) {
		// Inserts space before capital letters and numbers
		return (
			name
				?.replace(/([A-Z])/g, ' $1')
				.trim()
				.replace(/([a-z])(\d)/i, '$1 $2') || ''
		)
	}

	function select(label: string) {
		componentInput.value = label
		if ((document.activeElement as HTMLElement)?.blur) {
			;(document.activeElement as HTMLElement).blur()
		}
	}
</script>

<div class="relative">
	<input
		readonly
		value={formatName(componentInput.value)}
		bind:this={anchor}
		on:focus={getData}
		class="pr-8"
	/>
	{#if loading}
		<div class="center-center absolute right-2 top-1/2 transform -translate-y-1/2 p-0.5">
			<Loader2 class="animate-spin" size={16} />
		</div>
	{:else if componentInput.value}
		<button
			class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-200 rounded-full p-0.5"
			on:click|stopPropagation|preventDefault={() => (componentInput.value = undefined)}
			title="Clear"
			aria-label="Clear"
		>
			<X size="14" />
		</button>
	{/if}
</div>
{#if anchor}
	<Popup
		ref={anchor}
		options={{ placement: 'bottom' }}
		bind:open={openPopup}
		let:close
		transition={fade}
	>
		{#if !loading}
			<div class="max-w-xs shadow-[0_10px_40px_-5px_rgba(0,0,0,0.25)] bg-white rounded-md p-2">
				{#if filteredItems}
					<input
						on:keydown|stopPropagation
						bind:value={search}
						type="text"
						placeholder="Search"
						class="col-span-4 mb-2"
					/>
					<div class="grid gap-1 grid-cols-4 max-h-[300px] overflow-auto">
						{#each filteredItems as { label, icon }}
							{@const formatedLabel = formatName(label)}
							<button
								type="button"
								title={formatedLabel}
								on:click={() => {
									select(label)
									close()
								}}
								class="w-full center-center flex-col font-normal p-1 
									hover:bg-gray-100 focus:bg-gray-100 rounded duration-200 
									{label === componentInput.value ? 'text-blue-600 bg-blue-50 pointer-events-none' : ''}"
							>
								<svelte:component this={icon} size={22} />
								<span class="inline-block w-full text-[10px] ellipsize pt-0.5">
									{formatedLabel}
								</span>
							</button>
						{:else}
							<div class="col-span-4 text-center text-gray-700 text-sm p-2">
								No icons match your search
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center text-sm text-gray-600 p-2"> Couldn't load options </div>
				{/if}
			</div>
		{/if}
	</Popup>
{/if}
