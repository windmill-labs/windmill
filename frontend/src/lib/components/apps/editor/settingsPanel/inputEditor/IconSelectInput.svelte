<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { ClearableInput } from '../../../../common'
	import { AllIcons } from './icons'
	import type { ComputeConfig } from 'svelte-floating-ui'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	let loading = $state(false)
	let items: string[] | undefined = $state()
	let filteredItems: string[] | undefined = $state()
	let search = $state('')

	$effect.pre(() => {
		if (search) {
			filteredItems = items?.filter((item) => {
				return item.toLowerCase().includes(search.toLowerCase())
			})
		} else {
			filteredItems = items
		}
	})

	async function getData() {
		loading = true
		// @ts-ignore
		items = AllIcons

		loading = false
	}

	function select(label: string) {
		value = label

		const elem = document.activeElement as HTMLElement
		if (elem.blur) {
			elem.blur()
		}
	}

	interface Props {
		value?: string | undefined
		floatingConfig?: ComputeConfig
	}

	let {
		value = $bindable(''),
		floatingConfig = {
			strategy: 'absolute',
			placement: 'bottom-end'
		}
	}: Props = $props()
</script>

<Popover {floatingConfig} closeOnOtherPopoverOpen contentClasses="p-4">
	{#snippet trigger()}
		<div class="relative">
			<ClearableInput
				readonly
				{value}
				on:change={({ detail }) => (value = detail)}
				on:focus={getData}
				class="!pr-6"
			/>
			{#if loading}
				<div class="center-center absolute right-2 top-1/2 transform -translate-y-1/2 p-0.5">
					<Loader2 class="animate-spin" size={16} />
				</div>
			{/if}
		</div>
	{/snippet}
	{#snippet content({ close })}
		{#if !loading}
			{#if filteredItems}
				<div class="w-72">
					<input
						onkeydown={(event) => {
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
						{#each filteredItems as label}
							<button
								type="button"
								title={label}
								onclick={() => {
									select(label)
									close()
								}}
								class="w-full center-center flex-col font-normal p-1
												hover:bg-gray-100 focus:bg-gray-100 rounded duration-200 dark:hover:bg-frost-900 dark:focus:bg-frost-900
												{label === value ? 'text-blue-600 bg-blue-50 pointer-events-none' : ''}"
							>
								<!-- svelte-ignore a11y_missing_attribute -->
								<img
									class="dark:invert"
									loading="lazy"
									src="https://cdn.jsdelivr.net/npm/lucide-static@0.367.0/icons/{label}.svg"
								/>
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
	{/snippet}
</Popover>
