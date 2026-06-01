<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'

	let providerModel = $derived(
		$copilotSessionModel ??
			$copilotInfo.defaultModel ??
			$copilotInfo.aiModels[0] ?? {
				model: 'No model',
				provider: 'No provider'
			}
	)

	let multipleModels = $derived($copilotInfo.aiModels.length > 1)
</script>

{#if multipleModels}
	<DropdownV2
		items={() =>
			$copilotInfo.aiModels.map((m) => ({
				displayName: m.model,
				selected: m.model === providerModel.model,
				action: () => {
					$copilotSessionModel = m
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
