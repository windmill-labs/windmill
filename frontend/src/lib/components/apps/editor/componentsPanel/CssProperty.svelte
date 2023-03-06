<script lang="ts">
	import { X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { addWhitespaceBeforeCapitals } from '../../../../utils'
	import type { ComponentCssProperty } from '../../types'

	export let name: string
	export let value: ComponentCssProperty | undefined

	function reset(property: keyof ComponentCssProperty) {
		if (value) {
			value[property] = ''
		}
	}
</script>

<div class="text-sm font-semibold text-gray-500 capitalize pt-2">
	{addWhitespaceBeforeCapitals(name)}
</div>
{#if value}
	<div class="border-l border-gray-400/80 py-1 pl-3.5 mt-1 ml-0.5">
		{#if value.style !== undefined}
			<label class="block pb-2">
				<div class="text-xs font-medium pb-0.5"> Style </div>
				<div class="relative">
					<input type="text" class="!pr-7" bind:value={value.style} on:focus />
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
			</label>
		{/if}
		{#if value.class !== undefined}
			<label class="block">
				<div class="text-xs font-medium pb-0.5"> Tailwind classes </div>
				<div class="relative">
					<input type="text" class="!pr-7" bind:value={value.class} on:focus />
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
