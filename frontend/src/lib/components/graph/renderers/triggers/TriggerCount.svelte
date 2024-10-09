<script lang="ts">
	import { Loader2 } from 'lucide-svelte'

	export let count: number | undefined = undefined
</script>

{#if count && count > 0}
	<div class="absolute -right-2 -top-2">
		<div
			class="bg-nord-950 dark:bg-nord-400 rounded-md h-4 w-4 p-1 flex center-center text-primary-inverse text-2xs font-normal"
		>
			{#if count === undefined}
				<Loader2 class="animate-spin text-2xs" />
			{:else}
				{count}
			{/if}
		</div>
	</div>
{/if}
