<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import { aiChatManager } from './AIChatManager.svelte'
	let {
		allowedModes
	}: {
		allowedModes: {
			script: boolean
			flow: boolean
			navigator: boolean
		}
	} = $props()
</script>

<div class="min-w-0">
	<Popover
		disablePopup={Object.keys(allowedModes).filter((k) => allowedModes[k]).length < 2}
		class="max-w-full"
	>
		<svelte:fragment slot="trigger">
			<div
				class="text-tertiary text-xs flex flex-row items-center font-normal gap-0.5 border px-1 rounded-lg"
			>
				<span class={`truncate`}>
					{aiChatManager.mode} mode
				</span>
				{#if Object.keys(allowedModes).filter((k) => allowedModes[k]).length > 1}
					<div class="shrink-0">
						<ChevronDown size={16} />
					</div>
				{/if}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content" let:close>
			<div class="flex flex-col gap-1 p-1 min-w-24">
				{#each ['script', 'flow', 'navigator'] as possibleMode}
					{#if allowedModes[possibleMode]}
						<button
							class={twMerge(
								'text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal',
								aiChatManager.mode === possibleMode && 'bg-surface-hover'
							)}
							onclick={() => {
								aiChatManager.changeMode(possibleMode as 'script' | 'flow' | 'navigator')
								close()
							}}
						>
							{possibleMode} mode
						</button>
					{/if}
				{/each}
			</div>
		</svelte:fragment>
	</Popover>
</div>
