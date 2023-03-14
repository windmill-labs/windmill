<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import type { IntRange } from '../common'
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let element: `h${IntRange<1, 6>}` = 'h2'
	export let accordion = false
	let showContent = !accordion
</script>

<div class="border-b [&:has(button:hover)]:border-gray-400 duration-200 pb-1 mt-8 mb-2">
	{#if accordion}
		<button
			class="flex w-full justify-start items-center"
			on:click={() => (showContent = !showContent)}
		>
			<span class="rounded-full hover:bg-gray-100 focus:bg-gray-100 p-1 mr-1">
				<ChevronDown size={22} class="rotate-0 duration-300 {showContent ? '!-rotate-180' : ''}" />
			</span>
			<svelte:element this={element}>
				{title}
				<Tooltip scale={0.9} class="mb-0.5">
					{tooltip}
				</Tooltip>
			</svelte:element>
		</button>
	{:else}
		<svelte:element this={element}>
			{title}
			<Tooltip scale={0.9} class="mb-0.5">
				{tooltip}
			</Tooltip>
		</svelte:element>
	{/if}
</div>
{#if showContent}
	<div transition:slide={{ duration: 300 }}>
		<slot />
	</div>
{/if}
