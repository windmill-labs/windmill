<script lang="ts">
	import { LoaderIcon } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'

	interface Props {
		loading: boolean
	}

	let { loading }: Props = $props()
</script>

{#if loading}
	<Popover>
		<div class={'bg-blue-100 dark:bg-blue-400 transition-all p-1 rounded-component'}>
			<LoaderIcon size={14} class="animate-spin text-blue-800 dark:text-white" />
		</div>
		{#snippet text()}
			{#if loading}
				Refreshing...
			{:else}
				Refresh
			{/if}
		{/snippet}
	</Popover>
{/if}
