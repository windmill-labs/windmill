<script lang="ts">
	export let title: string | undefined = undefined
</script>

<div class="flex items-center justify-between flex-wra px-6 py-2 ">
	{#if title}
		<h3 class="text-sm font-bold text-gray-900">{title}</h3>
	{/if}
	<slot />
</div>
