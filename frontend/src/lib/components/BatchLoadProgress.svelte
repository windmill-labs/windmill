<script lang="ts">
	import Button from './common/button/Button.svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		loaded: number
		total: number
		itemsLabel: string
		batchSize?: number | null
		onBatchSizeChange: (batchSize: number) => void
		onStop: () => void
	}

	let { loaded, total, itemsLabel, batchSize = null, onBatchSizeChange, onStop }: Props = $props()

	let percent = $derived(total > 0 ? Math.round((Math.min(loaded, total) / total) * 100) : 0)
</script>

<div class="flex items-center gap-3 text-xs text-secondary">
	<span class="whitespace-nowrap shrink-0">Loading {itemsLabel}: {loaded} of {total}...</span>
	<div class="flex-1 min-w-8 bg-surface-hover rounded-full h-1.5">
		<div
			class="bg-blue-500 h-1.5 rounded-full transition-all duration-300"
			style="width: {percent}%"
		></div>
	</div>
	{#if batchSize != null}
		<span class="whitespace-nowrap shrink-0">Batch size:</span>
		<TextInput
			size="xs"
			class="!w-14 shrink-0 text-center"
			value={batchSize}
			inputProps={{
				type: 'number',
				min: 1,
				max: 1000,
				onchange: (e) => {
					const v = parseInt(e.currentTarget.value)
					if (v >= 1 && v <= 1000) {
						onBatchSizeChange(v)
					}
				}
			}}
		/>
	{/if}
	<Button size="xs" destructive onClick={onStop}>Stop</Button>
</div>
