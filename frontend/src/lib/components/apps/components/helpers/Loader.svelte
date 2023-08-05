<script lang="ts">
	import { Loader2 } from 'lucide-svelte'

	export let loading: boolean
</script>

{#if !loading}
	<slot />
{:else}
	<div class="w-full h-full">
		<div class="absolute inset-0 center-center flex-col bg-surface text-tertiary border">
			<Loader2 class="animate-spin" size={16} />
			<span class="text-xs mt-1">Loading</span>
		</div>
	</div>
{/if}
