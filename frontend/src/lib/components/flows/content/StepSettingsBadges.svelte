<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import type { FlowModule } from '$lib/gen'
	import { describeStepSettings } from '../flowStepSettings'

	interface Props {
		flowModule: FlowModule
	}

	let { flowModule }: Props = $props()

	// Same icons the graph badges a step with, so a setting looks the same wherever it
	// is surfaced.
	let configured = $derived(describeStepSettings(flowModule).filter((s) => s.configured))
</script>

{#if configured.length > 0}
	<div class="flex flex-row items-center gap-1 text-secondary">
		{#each configured as s (s.key)}
			{@const Icon = s.icon}
			<Popover notClickable>
				<Icon size={12} />
				{#snippet text()}
					{s.tooltip}
					<span class={s.summary.mono ? 'font-mono' : ''}>· {s.summary.text}</span>
				{/snippet}
			</Popover>
		{/each}
	</div>
{/if}
