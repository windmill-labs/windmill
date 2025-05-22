<script lang="ts">
	export let link: string | undefined = undefined
</script>

<div class="text-xs text-tertiary font-normal">
	<slot />
	{#if link}
		<a href={link} target="_blank">Learn more</a>
	{/if}
</div>
