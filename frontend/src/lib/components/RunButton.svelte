<script lang="ts">
	import { Button } from '$lib/components/common'
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'

	export let isLoading
	export let hideShortcut = false
	export let onRun: () => Promise<void>
	export let onCancel: () => Promise<void>
</script>

{#if !isLoading}
	<Button
		loading={isLoading}
		size="sm"
		color="dark"
		btnClasses="!px-2 !py-1"
		on:click={() => onRun()}
		shortCut={{ Icon: CornerDownLeft, hide: hideShortcut }}
	>
		Run
	</Button>
{:else}
	<Button
		size="sm"
		color="red"
		variant="border"
		btnClasses="!px-2 !py-1"
		on:click={() => onCancel()}
	>
		<Loader2 size={14} class="animate-spin mr-2" />
		Cancel
	</Button>
{/if}
