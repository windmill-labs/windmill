<script lang="ts">
	import { ChevronDown, Check } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import { CHAT_BAR_PILL, CHAT_BAR_PILL_STATIC } from './chatBarStyles'

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
				icon: m.model === providerModel.model ? Check : undefined,
				action: () => {
					$copilotSessionModel = m
					storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, m.model)
					storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, m.provider)
				}
			}))}
		placement="bottom-end"
		fixedHeight={false}
		class="min-w-0"
	>
		{#snippet buttonReplacement()}
			<div class={CHAT_BAR_PILL}>
				<span class="truncate">{providerModel.model}</span>
				<ChevronDown size={14} class="shrink-0" />
			</div>
		{/snippet}
	</DropdownV2>
{:else}
	<div class={CHAT_BAR_PILL_STATIC}>
		<span class="truncate">{providerModel.model}</span>
	</div>
{/if}
