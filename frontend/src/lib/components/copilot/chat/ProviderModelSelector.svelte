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
	let effortLabel = $derived(
		currentEffort ? currentEffort.charAt(0).toUpperCase() + currentEffort.slice(1) : 'Off'
	)

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
					// Carry the current effort onto the newly selected model (sticky).
					$copilotSessionModel = {
						...m,
						...(providerModel.reasoning ? { reasoning: providerModel.reasoning } : {})
					}
					storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, m.model)
					storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, m.provider)
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
			[REASONING_OFF, ...capability.levels].map((level) => ({
				displayName:
					level === REASONING_OFF ? 'Off' : level.charAt(0).toUpperCase() + level.slice(1),
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
