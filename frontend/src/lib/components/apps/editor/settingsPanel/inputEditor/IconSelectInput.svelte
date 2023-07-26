<script lang="ts">
	import type { StaticInput } from '../../../inputType'
	import { Loader2 } from 'lucide-svelte'
	import { ClearableInput, Popup } from '../../../../common'

	export let componentInput: StaticInput<string>

	let loading = false
	let items: { label: string; icon: any }[]
	let filteredItems: { label: string; icon: any }[]
	let search = ''

	$: if (search) {
		filteredItems = items.filter((item) => {
			return item.label.toLowerCase().includes(search.toLowerCase())
		})
	} else {
		filteredItems = items
	}

	async function getData() {
		if (items) {
		}
		loading = true
		const data = await import('lucide-svelte/dist/svelte/icons')

		filteredItems = items = Object.entries(data)
			.filter(([key]) => !(key.endsWith('Icon') || key.startsWith('Lucide')))
			.map(([key, icon]) => ({ label: key, icon }))

		loading = false
	}

	function select(label: string) {
		componentInput.value = label

		const elem = document.activeElement as HTMLElement
		if (elem.blur) {
			elem.blur()
		}
	}
</script>

<Popup
	let:close
	floatingConfig={{
		strategy: 'absolute',
		placement: 'bottom-end'
	}}
>
	<svelte:fragment slot="button">
		<div class="relative">
			<ClearableInput
				readonly
				value={componentInput.value}
				on:change={({ detail }) => (componentInput.value = detail)}
				on:focus={getData}
				class="!pr-6"
			/>
			{#if loading}
				<div class="center-center absolute right-2 top-1/2 transform -translate-y-1/2 p-0.5">
					<Loader2 class="animate-spin" size={16} />
				</div>
			{/if}
		</div>
	</svelte:fragment>
	{#if !loading}
		{#if filteredItems}
			<div class="w-72">
				<input
					on:keydown={(event) => {
						if (!['ArrowDown', 'ArrowUp'].includes(event.key)) {
							event.stopPropagation()
						}
					}}
					bind:value={search}
					type="text"
					placeholder="Search"
					class="col-span-4 mb-2"
				/>
				<div class="grid gap-1 grid-cols-4 max-h-[300px] overflow-auto">
					{#each filteredItems as { label, icon }}
						<button
							type="button"
							title={label}
							on:click={() => {
								select(label)
								close(null)
							}}
							class="w-full center-center flex-col font-normal p-1
									hover:bg-gray-100 focus:bg-gray-100 rounded duration-200 dark:hover:bg-frost-900 dark:focus:bg-frost-900
									{label === componentInput.value ? 'text-blue-600 bg-blue-50 pointer-events-none' : ''}"
						>
							<svelte:component this={icon} size={22} />
							<span class="inline-block w-full text-[10px] ellipsize pt-0.5">
								{label}
							</span>
						</button>
					{:else}
						<div class="col-span-4 text-center text-secondary text-sm p-2">
							No icons match your search
						</div>
					{/each}
				</div>
			</div>
		{:else}
			<div class="text-center text-sm text-secondary p-2"> Couldn't load options </div>
		{/if}
	{/if}
</Popup>
