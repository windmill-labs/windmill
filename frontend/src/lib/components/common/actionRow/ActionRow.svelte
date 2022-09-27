<script lang="ts">
	export let applyPageWidth = false
	let classes = 'max-w-6xl mx-auto px-4 sm:px-6 md:px-8'
</script>

<div class={'bg-white py-3 ' + $$props.class}>
	<div
		class={'w-full flex flex-wrap justify-between items-center gap-4 ' +
			(applyPageWidth ? classes : '')}
	>
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
		<div class="flex flex-wrap items-center gap-2">
			{#if $$slots.right}
				<slot name="right" />
			{/if}
		</div>
	</div>
</div>
