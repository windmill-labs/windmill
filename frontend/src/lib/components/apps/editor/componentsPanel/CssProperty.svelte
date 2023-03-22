<script lang="ts">
	import { Paintbrush2, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals } from '../../../../utils'
	import { Button } from '../../../common'
	import type { ComponentCssProperty } from '../../types'
	import QuickStyleMenu from './QuickStyleMenu.svelte'
	import type { StylePropertyKey } from './quickStyleProperties'

	export let name: string
	export let value: ComponentCssProperty = {}
	export let forceStyle: boolean = false
	export let forceClass: boolean = false
	export let quickStyleProperties: StylePropertyKey[] | undefined = undefined
	let isQuickMenuOpen = false

	function reset(property: Exclude<keyof ComponentCssProperty, 'quickStyling'>) {
		if (value) {
			value[property] = ''
		}
	}
</script>

<div class="sticky top-0 z-10 bg-white text-sm font-semibold text-gray-500 capitalize pt-2 pb-1">
	{addWhitespaceBeforeCapitals(name)}
</div>
{#if value}
	<div class="border-l border-gray-400/80 py-1 pl-3.5 ml-0.5">
		{#if value.style !== undefined || forceStyle}
			<div class="pb-2">
				<label class="block">
					<div class="text-xs font-medium pb-0.5"> Style </div>
					<div class="flex gap-1">
						<div class="relative grow">
							<input
								on:keydown|stopPropagation
								type="text"
								class="!pr-7"
								bind:value={value.style}
								on:focus
							/>
							{#if value?.style}
								<button
									transition:fade|local={{ duration: 100 }}
									class="absolute z-10 top-1.5 right-1 rounded-full p-1 duration-200 hover:bg-gray-200"
									aria-label="Remove styles"
									on:click|preventDefault|stopPropagation={() => reset('style')}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
						{#if quickStyleProperties?.length}
							<Button
								variant="border"
								color="light"
								size="xs"
								btnClasses="!p-1 !w-[34px] !h-[34px]"
								aria-label="Toggle quick style menu"
								title="Toggle quick style menu"
								on:click={() => (isQuickMenuOpen = !isQuickMenuOpen)}
							>
								<Paintbrush2 size={18} />
							</Button>
						{/if}
					</div>
				</label>
				{#if quickStyleProperties?.length && isQuickMenuOpen}
					<div transition:fade|local={{ duration: 200 }} class="w-full pt-1">
						<QuickStyleMenu bind:value={value.style} properties={quickStyleProperties} />
					</div>
				{/if}
			</div>
		{/if}
		{#if value.class !== undefined || forceClass}
			<label class="block">
				<div class="text-xs font-medium pb-0.5"> Tailwind classes </div>
				<div class="relative">
					<input
						on:keydown|stopPropagation
						type="text"
						class="!pr-7"
						bind:value={value.class}
						on:focus
					/>
					{#if value?.class}
						<button
							transition:fade|local={{ duration: 100 }}
							class="absolute z-10 top-1.5 right-1 rounded-full p-1 duration-200 hover:bg-gray-200"
							aria-label="Remove classes"
							on:click|preventDefault|stopPropagation={() => reset('class')}
						>
							<X size={14} />
						</button>
					{/if}
				</div>
			</label>
		{/if}
	</div>
{/if}
