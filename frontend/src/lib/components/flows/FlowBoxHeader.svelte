<script lang="ts">
	export let title: string | undefined = undefined
</script>

<div class="flex items-center justify-between flex-wra px-6 py-4 border-b border-gray-300">
	{#if title}
		<h3 class="text-lg leading-6 font-bold text-gray-900">{title}</h3>
	{/if}
	<slot />
</div>
