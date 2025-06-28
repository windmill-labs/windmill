<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/stores'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getModifierKey } from '$lib/utils'
	import { WandSparkles } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		togglePanel,
		opened,
		class: className
	}: { togglePanel: () => void; opened?: boolean; class?: string } = $props()
</script>

{#snippet button(onClick: () => void)}
	<Button
		color="light"
		variant="border"
		size="xs2"
		on:click={onClick}
		startIcon={{ icon: WandSparkles }}
		iconOnly
		btnClasses={twMerge(
			'!text-violet-800 dark:!text-violet-400 border border-gray-200 dark:border-gray-600 bg-surface h-[28px] w-[32px] rounded-md py-1 px-1',
			opened ? 'bg-surface-selected' : '',
			className
		)}
	>
		AI Panel
	</Button>
{/snippet}

{#if $copilotInfo.enabled}
	<DarkPopover>
		{#snippet text()}
			<div class="flex flex-row gap-1">
				Show the AI Panel.

				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{getModifierKey()}L
				</div>
			</div>
		{/snippet}
		{@render button(togglePanel)}
	</DarkPopover>
{:else}
	<Popover placement="bottom">
		{#snippet trigger()}
			{@render button(() => {
				togglePanel()
			})}
		{/snippet}
		{#snippet content()}
			<div class="block text-primary p-4">
				<p class="text-sm"
					>Enable Windmill AI in the <a
						href="{base}/workspace_settings?tab=ai"
						target="_blank"
						class="inline-flex flex-row items-center gap-1"
						>workspace settings <ExternalLink size={16} /></a
					></p
				>
			</div>
		{/snippet}
	</Popover>
{/if}
