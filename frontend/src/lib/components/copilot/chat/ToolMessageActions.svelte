<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ExternalLink } from 'lucide-svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
	import type { ToolDisplayAction } from './shared'

	interface Props {
		actions: ToolDisplayAction[]
	}

	let { actions }: Props = $props()
	let runningActionId: string | undefined = $state(undefined)

	async function handleAction(action: ToolDisplayAction) {
		if (runningActionId) {
			return
		}
		runningActionId = action.id
		try {
			await runToolDisplayAction(action)
		} finally {
			runningActionId = undefined
		}
	}
</script>

{#if actions.length > 0}
	<div class="pt-1 flex flex-row flex-wrap justify-center gap-1">
		{#each actions as action (action.id)}
			<Button
				size="xs"
				variant="accent"
				title={action.label}
				loading={runningActionId === action.id}
				disabled={runningActionId !== undefined && runningActionId !== action.id}
				startIcon={{ icon: ExternalLink }}
				onClick={() => handleAction(action)}
			>
				{action.label}
			</Button>
		{/each}
	</div>
{/if}
