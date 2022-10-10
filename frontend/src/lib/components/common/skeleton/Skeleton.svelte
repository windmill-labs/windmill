<script lang="ts">
	import { fade } from 'svelte/transition'
	import { skeleton } from '../../../directives'
	import { HEIGHT_UNIT, type SkeletonLayout } from './model'
	import SkeletonElement from './SkeletonElement.svelte'

	export let layout: SkeletonLayout
	export let loading = true
	export let overlay = false
	export let delay = 200

	let wrapper: HTMLElement
	$: maxWidth =
		overlay && wrapper ? wrapper.parentElement?.getBoundingClientRect()?.width : undefined
	$: leftPadding = overlay && wrapper ? window.getComputedStyle(wrapper).paddingLeft : undefined
	$: rightPadding = overlay && wrapper ? window.getComputedStyle(wrapper).paddingRight : undefined
</script>

{#if loading}
	<div
		in:fade={{ duration: 1000, delay }}
		out:fade={{ duration: 100 }}
		bind:this={wrapper}
		use:skeleton={true}
		class="flex flex-col overflow-hidden {overlay
			? 'absolute w-full h-full z-[1000]'
			: ''} {$$props.class}"
		style="max-width: {maxWidth ? maxWidth + 'px' : 'none'}; padding-left: {leftPadding ||
			0}; padding-right: {rightPadding || 0}"
	>
		{#each layout as row}
			<div class="flex justify-between items-start gap-4">
				{#if typeof row === 'number'}
					<div style="height: {row * HEIGHT_UNIT}px;" class="!animate-none !bg-transparent" />
				{:else if Array.isArray(row)}
					{#each row as el}
						{@const element = typeof el === 'number' ? { h: el, w: 100 / row.length, minW: 0 } : el}
						<SkeletonElement {element} />
					{/each}
				{:else}
					{@const { elements, h } = row}
					{#each new Array(elements) as _}
						<SkeletonElement element={{ h, w: 100 / elements }} />
					{/each}
				{/if}
			</div>
		{/each}
	</div>
{/if}
