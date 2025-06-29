<script lang="ts">
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		loading: boolean
		children?: import('svelte').Snippet
	}

	let { loading, children }: Props = $props()
</script>

{#if !loading}
	{@render children?.()}
{:else}
	<div class="w-full h-full">
		<div class="absolute inset-0 center-center flex-col bg-surface text-tertiary border">
			<Loader2 class="animate-spin" size={16} />
			<span class="text-xs mt-1">Loading</span>
		</div>
	</div>
{/if}
