<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME
	} from '$lib/stores'
	import { storeLocalSetting } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
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

<div class="min-w-0">
	<Popover disablePopup={!multipleModels} class="max-w-full">
		{#snippet trigger()}
			<div class="text-primary text-xs flex flex-row items-center font-normal gap-0.5">
				<span class={`truncate ${multipleModels ? '' : 'pr-2'}`}>{providerModel.model}</span>
				{#if multipleModels}
					<div class="shrink-0">
						<ChevronDown size={16} />
					</div>
				{/if}
			</div>
		{/snippet}
		{#snippet content({ close })}
			<div class="flex flex-col gap-1 p-1 min-w-24">
				{#each $copilotInfo.aiModels as providerModel}
					<button
						class={twMerge(
							'text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal',
							providerModel.model === $copilotSessionModel?.model && 'bg-surface-hover'
						)}
						onclick={() => {
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
		{/snippet}
	</Popover>
</div>
