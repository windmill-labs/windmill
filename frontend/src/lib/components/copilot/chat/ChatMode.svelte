<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		mode = $bindable(),
		allowedModes
	}: {
		mode: 'script' | 'flow'
		allowedModes: {
			script: boolean
			flow: boolean
		}
	} = $props()
</script>

<div class="min-w-0">
	<Popover disablePopup={!allowedModes.script || !allowedModes.flow} class="max-w-full">
		<svelte:fragment slot="trigger">
			<div
				class="text-tertiary text-xs flex flex-row items-center font-normal gap-0.5 border px-1 rounded-lg"
			>
				<span class={`truncate`}>
					{mode} mode
				</span>
				{#if allowedModes.script && allowedModes.flow}
					<div class="shrink-0">
						<ChevronDown size={16} />
					</div>
				{/if}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content" let:close>
			<div class="flex flex-col gap-1 p-1 min-w-24">
				{#each ['script', 'flow'] as possibleMode}
					<button
						class={twMerge(
							'text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal',
							mode === possibleMode && 'bg-surface-hover'
						)}
						onclick={() => {
							mode = possibleMode as 'script' | 'flow'
							close()
						}}
					>
						{possibleMode} mode
					</button>
				{/each}
			</div>
		</svelte:fragment>
	</Popover>
</div>
