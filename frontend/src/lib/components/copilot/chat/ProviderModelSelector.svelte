<script lang="ts">
	import { ChevronDown, Brain } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME,
		COPILOT_SESSION_REASONING_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import type { AIProvider } from '$lib/gen'
	import {
		getReasoningCapability,
		resolveEffectiveReasoning,
		REASONING_OFF,
		type ReasoningProviderModel
	} from '../reasoningRegistry'

	let providerModel = $derived(
		($copilotSessionModel ??
			$copilotInfo.defaultModel ??
			$copilotInfo.aiModels[0] ?? {
				model: 'No model',
				provider: 'No provider'
			}) as ReasoningProviderModel
	)

	let multipleModels = $derived($copilotInfo.aiModels.length > 1)

	let capability = $derived(
		getReasoningCapability(providerModel.provider as AIProvider, providerModel.model)
	)
	// Effective effort accounts for the default-on level on capable models.
	let currentEffort = $derived(resolveEffectiveReasoning(providerModel))
	// Effort tokens are shown verbatim (lowercase), matching the provider-native values.
	let effortLabel = $derived(currentEffort ?? REASONING_OFF)

	function selectReasoning(value: string) {
		const reasoning = value === REASONING_OFF ? REASONING_OFF : value
		$copilotSessionModel = {
			...providerModel,
			provider: providerModel.provider as AIProvider,
			reasoning
		}
		// Pin the current model selection so the reasoning choice persists with it.
		storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, providerModel.model)
		storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, providerModel.provider)
		storeLocalSetting(COPILOT_SESSION_REASONING_SETTING_NAME, reasoning)
	}
</script>

{#if multipleModels}
	<DropdownV2
		items={() =>
			$copilotInfo.aiModels.map((m) => ({
				displayName: m.model,
				selected: m.model === providerModel.model,
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
					$copilotSessionModel = { ...m, ...(keep ? { reasoning: carried } : {}) }
					storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, m.model)
					storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, m.provider)
					storeLocalSetting(COPILOT_SESSION_REASONING_SETTING_NAME, keep ? carried : undefined)
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
