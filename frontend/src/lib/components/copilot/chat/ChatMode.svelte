<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
</script>

<div class="min-w-0">
	<Popover
		disablePopup={Object.keys(aiChatManager.allowedModes).filter(
			(k) => aiChatManager.allowedModes[k]
		).length < 2}
		class="max-w-full"
	>
		<svelte:fragment slot="trigger">
			<div
				class="text-tertiary text-xs flex flex-row items-center font-normal gap-0.5 border px-1 rounded-lg"
			>
				<span class={`truncate`}>
					{aiChatManager.mode.charAt(0).toUpperCase() + aiChatManager.mode.slice(1)} mode
				</span>
				{#if Object.keys(aiChatManager.allowedModes).filter((k) => aiChatManager.allowedModes[k]).length > 1}
					<div class="shrink-0">
						<ChevronDown size={16} />
					</div>
				{/if}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content" let:close>
			<div class="flex flex-col gap-1 p-1 min-w-24">
				{#each Object.values(AIMode) as possibleMode}
					{#if aiChatManager.allowedModes[possibleMode]}
						<button
							class={twMerge(
								'text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal',
								aiChatManager.mode === possibleMode && 'bg-surface-hover'
							)}
							onclick={() => {
								aiChatManager.changeMode(possibleMode)
								close()
							}}
						>
							{possibleMode.charAt(0).toUpperCase() + possibleMode.slice(1)} mode
						</button>
					{/if}
				{/each}
			</div>
		</svelte:fragment>
	</Popover>
</div>
