<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { ShieldAlert } from 'lucide-svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { FlowEditorContext } from '../types'
	import { canAddErrorHandling, errorBranchesAfter } from '../errorHandling'
	import { addErrorHandling } from '../errorHandlingInsert'
	import StepIdBadge from './StepIdBadge.svelte'

	interface Props {
		stepId: string
		continueOnError: boolean
		/** Classes for the button / "Errors handled by" row. */
		class?: string
	}

	let { stepId, continueOnError, class: rowClass = '' }: Props = $props()

	// Outside the flow editor (run views) there is nothing to insert into.
	const ctx = getContext<FlowEditorContext | undefined>('FlowEditorContext')

	// Recognised from the flow, never stored: see errorHandling.ts.
	const canAdd = $derived(ctx ? canAddErrorHandling(ctx.flowStore.val.value, stepId) : false)
	const handler = $derived(ctx ? errorBranchesAfter(ctx.flowStore.val.value, stepId) : undefined)
</script>

{#if ctx}
	{#if canAdd && !handler}
		<div class={twMerge('flex items-center gap-2 text-xs text-secondary', rowClass)}>
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: ShieldAlert }}
				title={continueOnError
					? 'Add a branch after this step with an On success path and an On error path.'
					: 'Turn on Continue on error and add a branch after this step with an On success path and an On error path.'}
				onClick={() => addErrorHandling(ctx, stepId)}
			>
				Add error handling
			</Button>
		</div>
	{:else if handler && continueOnError}
		{@const handlerId = handler.id}
		<div class={twMerge('flex items-center gap-2 text-xs text-secondary', rowClass)}>
			<span>Errors handled by</span>
			<StepIdBadge id={handlerId} />
			<Button
				variant="default"
				unifiedSize="xs"
				title="Open the error handling branches"
				onClick={() => ctx.selectionManager.selectId(handlerId)}
			>
				Open
			</Button>
		</div>
	{:else if handler}
		<Alert type="info" title="Error branches never run" size="xs">
			<StepIdBadge id={handler.id} /> handles this step's errors, but with Continue on error off the
			flow stops at this step, so its error branches never run.
		</Alert>
	{/if}
{/if}
