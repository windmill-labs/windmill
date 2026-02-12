<script lang="ts">
	import { slide } from 'svelte/transition'

	interface Props {
		error: string
	}

	let { error }: Props = $props()
</script>

{#if error}
	<div transition:slide={{ duration: 150 }} class="text-red-600 dark:text-red-400 text-xs mt-1">
		{error}
	</div>
{/if}
