<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME,
		copilotInfo,
		copilotSessionModel
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'

	$: providerModel = $copilotSessionModel ??
		$copilotInfo.defaultModel ??
		$copilotInfo.aiModels[0] ?? {
			model: 'No model',
			provider: 'No provider'
		}
</script>

<div class="min-w-0">
	<Popover disablePopup={$copilotInfo.aiModels.length <= 1} class="max-w-full">
		<svelte:fragment slot="trigger">
			<div class="text-tertiary text-xs flex flex-row items-center gap-0.5 font-normal">
				<span class="truncate">{providerModel.model}</span>
				{#if $copilotInfo.aiModels.length > 1}
					<div class="shrink-0">
						<ChevronDown size={16} />
					</div>
				{/if}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content" let:close>
			<div class="flex flex-col gap-1 p-1 min-w-24">
				{#each $copilotInfo.aiModels.filter((m) => m.model !== providerModel.model) as providerModel}
					<button
						class="text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal"
						on:click={() => {
							$copilotSessionModel = providerModel
							storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, providerModel.model)
							storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, providerModel.provider)
							close()
						}}
					>
						{providerModel.model}
					</button>
				{/each}
			</div>
		</svelte:fragment>
	</Popover>
</div>
