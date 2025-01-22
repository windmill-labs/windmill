<script lang="ts">
	import { Button } from '$lib/components/common'
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'

	export let runLoading = false
	export let hideShortcut = false
	export let onRun: () => Promise<void>
	export let onCancel: () => Promise<void>

</script>

{#if !runLoading}
	<Button
		loading={runLoading}
		size="xs"
		color="dark"
		btnClasses="!px-2 !py-1"
		on:click={async () => {
			runLoading = true
			await onRun()
			runLoading = false
		}}
		shortCut={{ Icon: CornerDownLeft, hide: hideShortcut }}
	>
		Run
	</Button>
{:else}
	<Button
		size="xs"
		color="red"
		variant="border"
		btnClasses="!px-2 !py-1.5"
		on:click={async () => {
			await onCancel()
			runLoading = false
		}}
	>
		<Loader2 size={14} class="animate-spin mr-2" />
		Cancel
	</Button>
{/if}
