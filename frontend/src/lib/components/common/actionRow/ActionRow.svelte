<script lang="ts">
	export let applyPageWidth = false
	let classes = 'max-w-6xl mx-auto px-4 sm:px-6 md:px-8'
</script>

<div class={'bg-white py-3 ' + $$props.class}>
	<div
		class={'w-full flex flex-wrap justify-between items-center gap-4 ' +
			(applyPageWidth ? classes : '')}
	>
		{#if $$slots.left}
			<div class="flex flex-wrap items-center gap-2">
				<slot name="left" />
			</div>
		{/if}
		{#if $$slots.middle}
			<div class="flex flex-wrap items-center gap-2">
				<slot name="middle" />
			</div>
		{/if}
		{#if $$slots.right}
			<div class="flex flex-wrap items-center gap-2">
				<slot name="right" />
			</div>
		{/if}
	</div>
</div>
