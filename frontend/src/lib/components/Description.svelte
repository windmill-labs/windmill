<!-- @migration-task Error while migrating Svelte code: $$props is used together with named props in a way that cannot be automatically migrated. -->
<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let link: string | undefined = undefined
</script>

<div class={twMerge('text-xs text-primary font-normal', $$props.class)}>
	<slot />
	{#if link}
		<a href={link} target="_blank">Learn more</a>
	{/if}
</div>
