<script lang="ts">
	import { Loader2 } from 'lucide-svelte'

	export let count: number | undefined = undefined
</script>

{#if count && count > 0}
	<div class="absolute -right-2 -top-2">
		<div
			class="bg-surface-secondary font-bold border border-surface-border rounded-md h-4 w-4 p-1 flex center-center text-primary text-xs"
		>
			{#if count === undefined}
				<Loader2 class="animate-spin text-2xs" />
			{:else}
				{count}
			{/if}
		</div>
	</div>
{/if}
