<script lang="ts">
	import { ArrowRight } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'

	type Props = {
		targetTable: string
		targetColumn: string
		onGoToRow: () => void
	}
	let { targetTable, targetColumn, onGoToRow }: Props = $props()
</script>

<div
	class="flex flex-col gap-1.5 rounded-md border bg-surface-tertiary p-2 text-xs shadow-md"
	data-testid="db-fk-tooltip"
>
	<div class="text-secondary">
		References <span class="font-mono text-primary">{targetTable}.{targetColumn}</span>
	</div>
	<Button unifiedSize="xs" variant="default" startIcon={{ icon: ArrowRight }} onClick={onGoToRow}>
		Go to row
	</Button>
</div>
