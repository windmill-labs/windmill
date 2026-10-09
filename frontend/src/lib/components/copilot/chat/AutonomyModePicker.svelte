<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { AIAutonomyMode } from './AIChatManager.svelte'
	import {
		autonomyModeOption,
		availableAutonomyModeOptions,
		resolveAutonomyMode,
		type AutonomyAvailability
	} from './autonomyModes'

	let {
		mode,
		availability,
		onChange
	}: {
		mode: AIAutonomyMode
		availability: AutonomyAvailability
		onChange: (mode: AIAutonomyMode) => void
	} = $props()

	const effective = $derived(resolveAutonomyMode(mode, availability))
	const option = $derived(autonomyModeOption(effective))
</script>

<DropdownV2
	items={() =>
		availableAutonomyModeOptions(availability).map((o) => ({
			displayName: o.label,
			selected: effective === o.mode,
			action: () => onChange(o.mode)
		}))}
	placement="bottom-start"
	fixedHeight={false}
	customWidth={240}
>
	{#snippet buttonReplacement()}
		<Button
			nonCaptureEvent
			unifiedSize="2xs"
			variant="default"
			title={option.tooltip(availability)}
			btnClasses={option.triggerClass ?? ''}
			startIcon={{ icon: option.icon, classes: option.iconColor }}
			endIcon={{ icon: ChevronDown }}
		>
			{option.shortLabel ?? option.label}
		</Button>
	{/snippet}
</DropdownV2>
