<script lang="ts">
	import { ChevronDown, Brain } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import type { AIProvider, AIProviderModel } from '$lib/gen'
	import {
		getReasoningCapability,
		resolveEffectiveReasoning,
		REASONING_OFF,
		type ReasoningProviderModel
	} from '../reasoningRegistry'

	interface Props {
		/** The model a request would send, as resolved from the pick. */
		model: ReasoningProviderModel | undefined
		onPick: (pick: ReasoningProviderModel) => void
		/** The models offered, when they are not the tab's workspace's (an AI session's). */
		models?: AIProviderModel[]
	}

	let { model, onPick, models }: Props = $props()
	let offered = $derived(models ?? $copilotInfo.aiModels)

	let providerModel = $derived(
		(model ?? {
			model: 'No model',
			provider: 'No provider'
		}) as ReasoningProviderModel
	)

	let multipleModels = $derived(offered.length > 1)

	let capability = $derived(
		getReasoningCapability(providerModel.provider as AIProvider, providerModel.model)
	)
	// Effective effort accounts for the default-on level on capable models.
	let currentEffort = $derived(resolveEffectiveReasoning(providerModel))
	// Effort tokens are shown verbatim (lowercase), matching the provider-native values.
	let effortLabel = $derived(currentEffort ?? REASONING_OFF)

	function selectReasoning(value: string) {
		// Pin the current model selection so the reasoning choice persists with it.
		onPick({ ...providerModel, reasoning: value })
	}
</script>

{#if multipleModels}
	<DropdownV2
		items={() =>
			offered.map((m) => ({
				displayName: m.model,
				selected: m.model === providerModel.model && m.provider === providerModel.provider,
				action: () => {
					// Carry the effort onto the new model only if it supports that level
					// ('off' only where the model can truly disable); otherwise drop it so
					// the model's default applies.
					const carried = providerModel.reasoning
					const cap = getReasoningCapability(m.provider, m.model)
					const keep =
						carried === REASONING_OFF
							? cap.canDisable
							: carried !== undefined && cap.levels.includes(carried)
					onPick({ ...m, ...(keep ? { reasoning: carried } : {}) })
				}
			}))}
		placement="bottom-end"
		fixedHeight={false}
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				endIcon={{ icon: ChevronDown }}
				btnClasses="max-w-[160px] text-secondary font-normal"
			>
				<span class="truncate">{providerModel.model}</span>
			</Button>
		{/snippet}
	</DropdownV2>
{:else}
	<Button unifiedSize="2xs" variant="subtle" btnClasses="max-w-[160px] text-secondary font-normal">
		<span class="truncate">{providerModel.model}</span>
	</Button>
{/if}

{#if capability.supported}
	<DropdownV2
		items={() =>
			[...(capability.canDisable ? [REASONING_OFF] : []), ...capability.levels].map((level) => ({
				displayName: level,
				selected:
					level === REASONING_OFF
						? providerModel.reasoning === REASONING_OFF
						: currentEffort === level && providerModel.reasoning !== REASONING_OFF,
				action: () => selectReasoning(level)
			}))}
		placement="bottom-end"
		fixedHeight={false}
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				startIcon={{ icon: Brain }}
				endIcon={{ icon: ChevronDown }}
				btnClasses="text-secondary font-normal"
				title="Reasoning effort"
			>
				<span class="truncate">{effortLabel}</span>
			</Button>
		{/snippet}
	</DropdownV2>
{/if}
