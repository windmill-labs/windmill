<script lang="ts">
	interface Props {
		applyPageWidth?: boolean
		stickToTop?: boolean
		class?: string | undefined
		left?: import('svelte').Snippet
		middle?: import('svelte').Snippet
		right?: import('svelte').Snippet
	}

	let {
		applyPageWidth = false,
		stickToTop = false,
		class: clazz = '',
		left,
		middle,
		right
	}: Props = $props()
	let wide = $derived(applyPageWidth ? 'max-w-7xl mx-auto px-4 sm:px-6 md:px-8' : '')

	let scrollY = $state(0)
</script>

<svelte:window bind:scrollY />

<div
	class={'bg-surface py-3 ' +
		(stickToTop
			? 'lg:sticky lg:top-0 z-[500] border-b border-gray-200 border-opacity-0 duration-300 ' +
				(scrollY >= 30 ? 'border-opacity-100 ' : '')
			: '') +
		(clazz || '')}
>
	<div class={'w-full flex flex-wrap justify-between items-center gap-4 ' + wide}>
		<div class="flex flex-wrap items-center gap-2">
			{#if left}
				{@render left?.()}
			{/if}
		</div>
		<div class="flex flex-wrap items-center gap-2">
			{#if middle}
				{@render middle?.()}
			{/if}
		</div>
		<div class="flex flex-wrap items-center gap-2 lg:gap-4">
			{#if right}
				{@render right?.()}
			{/if}
		</div>
	</div>
</div>
