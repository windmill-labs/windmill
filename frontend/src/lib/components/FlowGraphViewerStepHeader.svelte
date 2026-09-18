<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { Badge, Button } from './common'
	import { ArrowLeft } from 'lucide-svelte'

	interface Props {
		/** A module, or the graph's pseudo-nodes by id (`Input`, `Result`). */
		stepDetail: FlowModule | string
		/** Given, the row starts with a back control; the caller decides where back leads. */
		onBack?: () => void
	}

	let { stepDetail, onBack = undefined }: Props = $props()

	const module = $derived(typeof stepDetail === 'string' ? undefined : stepDetail)
	// The error handler and the preprocessor are named by their role, not by an id badge.
	const showId = $derived(
		module?.id !== undefined && module.id !== 'failure' && module.id !== 'preprocessor'
	)

	const title = $derived.by((): string => {
		if (typeof stepDetail === 'string') {
			if (stepDetail === 'Input') return 'Flow inputs'
			if (stepDetail === 'Result') return 'Result'
			return stepDetail
		}
		if (stepDetail.summary) return stepDetail.summary
		if (stepDetail.id === 'failure') return 'Error handler'
		if (stepDetail.id === 'preprocessor') return 'Preprocessor'
		const v = stepDetail.value
		switch (v?.type) {
			case 'identity':
				return 'Identity'
			case 'forloopflow':
				return (
					'For loop' +
					(v.parallel ? ' (parallel)' : '') +
					(v.skip_failures ? ' (skip failures)' : '') +
					(v.squash ? ' (squash)' : '')
				)
			case 'whileloopflow':
				return (
					'While loop' + (v.skip_failures ? ' (skip failures)' : '') + (v.squash ? ' (squash)' : '')
				)
			case 'branchall':
				return 'Run all branches' + (v.parallel ? ' (parallel)' : '')
			case 'branchone':
				return 'Run one branch'
			case 'flow':
				return 'Inner flow'
			case 'rawscript':
				return `Inline ${v.language} script`
			case 'script':
				return 'Workspace script'
			case 'aiagent':
				return 'AI Agent'
			default:
				return stepDetail.id
		}
	})
</script>

<!-- -top-2: the row pins at the scroll container's content edge, and FlowGraphViewerStep pads
     its root by that much, so at top-0 the body would show through the padding above the row. -->
<div class="sticky -top-2 z-10 flex w-full items-center gap-2 bg-surface py-2">
	{#if onBack}
		<Button
			unifiedSize="sm"
			variant="subtle"
			iconOnly
			startIcon={{ icon: ArrowLeft }}
			title="Back to the flow graph"
			onclick={onBack}
		/>
	{/if}
	{#if showId && module}
		<Badge color="indigo">{module.id}</Badge>
	{/if}
	<span class="min-w-0 truncate text-sm font-semibold text-emphasis" {title}>{title}</span>
</div>
