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

	$: multipleModels = $copilotInfo.aiModels.length > 1
</script>

<div class="min-w-0">
	<Popover disablePopup={!multipleModels} class="max-w-full">
		<svelte:fragment slot="trigger">
			<div class="text-tertiary text-xs flex flex-row items-center font-normal gap-0.5">
				<span class={`truncate ${multipleModels ? '' : 'pr-2'}`}>{providerModel.model}</span>
				{#if multipleModels}
					<div class="shrink-0 pr-1">
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
