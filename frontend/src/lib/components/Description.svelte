<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let link: string | undefined = undefined
</script>

<div class={twMerge('text-xs text-tertiary font-normal opacity-80', $$props.class)}>
	<slot />
	{#if link}
		<a href={link} target="_blank">Learn more</a>
	{/if}
</div>
