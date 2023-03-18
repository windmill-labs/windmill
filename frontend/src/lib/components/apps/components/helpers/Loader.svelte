<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { fade } from 'svelte/transition'

	export let loading: boolean
</script>

{#if !loading}
	<slot />
{:else}
	<div class="w-full h-full">
		<div
			out:fade|local={{ duration: 50 }}
			class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
		>
			<Loader2 class="animate-spin" size={16} />
			<span class="text-xs mt-1">Loading</span>
		</div>
	</div>
{/if}
