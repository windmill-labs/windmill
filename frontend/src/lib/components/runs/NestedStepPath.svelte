<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { ExternalLink } from 'lucide-svelte'
	import StepIdBadge from '../flows/content/StepIdBadge.svelte'

	interface Props {
		/** Step ids from the page's run down to the nested run holding the approval step. */
		path: string[]
		/** The nested run, whose own page shows its steps, graph and past approvals. */
		runId: string
		workspace: string | undefined
	}

	let { path, runId, workspace }: Props = $props()
</script>

<!-- The outermost step only, the one the run's graph shows: any depth keeps the row one chip wide. -->
<Popover
	openOnHover
	debounceDelay={140}
	placement="bottom-start"
	triggerAttrs={{ 'aria-label': `Inside step ${path.join(' › ')}` }}
	class="flex items-center gap-1 rounded-md px-1 text-hint hover:bg-surface-hover"
>
	{#snippet trigger()}
		in <StepIdBadge id={path[0]} />{path.length > 1 ? ' › …' : ''}
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-2 p-3 text-xs max-w-[calc(100vw-32px)]">
			<span class="flex flex-wrap items-center gap-1 text-secondary">
				Inside step
				{#each path as id, i (i)}
					{#if i > 0}<span class="text-hint">›</span>{/if}
					<StepIdBadge {id} />
				{/each}
			</span>
			<Button
				variant="default"
				unifiedSize="sm"
				href="/run/{runId}?workspace={workspace ?? ''}"
				target="_blank"
				endIcon={{ icon: ExternalLink }}
			>
				Open its run
			</Button>
		</div>
	{/snippet}
</Popover>
