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
					<!-- A badge only says a setting is on; its value is what the reader came for,
					     and reaching it otherwise means opening the panel. -->
					{s.tooltip}
					<span class={s.summary.mono ? 'font-mono' : ''}>· {s.summary.text}</span>
				{/snippet}
			</Popover>
		{/each}
	</div>
{/if}
