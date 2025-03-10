<script lang="ts">
	import { HEIGHT_UNIT, type SkeletonLayout } from './model'
	import SkeletonElement from './SkeletonElement.svelte'
	import { onMount } from 'svelte'

	export let layout: SkeletonLayout
	export let loading = true
	export let overlay = false

	export let mounted = false
	onMount(() => {
		setTimeout(() => {
			mounted = true
		}, 10)
	})
</script>

{#if loading}
	<div class="relative flex justify-center">
		<div
			class="flex grow flex-col overflow-hidden transition-[opacity] duration-1000 opacity-0 {mounted
				? 'opacity-100'
				: ''} {overlay ? 'absolute w-full h-full z-[1000]' : ''} {$$props.class}"
		>
			{#each layout as row}
				<div class="flex justify-between items-start gap-4">
					{#if typeof row === 'number'}
						<div style="height: {row * HEIGHT_UNIT}px;"></div>
					{:else if Array.isArray(row)}
						{#each row as el}
							{@const element =
								typeof el === 'number' ? { h: el, w: 100 / row.length, minW: 0 } : el}
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
	</div>
{/if}
