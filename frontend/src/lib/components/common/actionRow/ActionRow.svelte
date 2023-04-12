<script lang="ts">
	export let applyPageWidth = false
	export let stickToTop = false

	$: wide = applyPageWidth ? 'max-w-6xl mx-auto px-4 sm:px-6 md:px-8' : ''

	let scrollY = 0
</script>

<svelte:window bind:scrollY />

<div
	class={'bg-white py-3 ' +
		(stickToTop
			? 'lg:sticky lg:top-0 z-[500] border-b border-gray-200 border-opacity-0 duration-300 ' +
			  (scrollY >= 30 ? 'border-opacity-100 ' : '')
			: '') +
		($$props.class || '')}
>
	<div class={'w-full flex flex-wrap justify-between items-center gap-4 ' + wide}>
		<div class="flex flex-wrap items-center gap-2">
			{#if $$slots.left}
				<slot name="left" />
			{/if}
		</div>
		<div class="flex flex-wrap items-center gap-2">
			{#if $$slots.middle}
				<slot name="middle" />
			{/if}
		</div>
		<div class="flex flex-wrap items-center gap-2 lg:gap-4">
			{#if $$slots.right}
				<slot name="right" />
			{/if}
		</div>
	</div>
</div>
