<script module lang="ts">
	export function flowAIBtnClasses(state: 'default' | 'selected' | 'green' = 'default') {
		return twMerge(
			['selected', 'default'].includes(state) ? 'text-ai !border-border-ai hover:bg-ai/15' : '',
			{
				default: '',
				selected: 'bg-ai/10',
				green:
					'bg-green-50 hover:bg-green-50 dark:bg-green-400/15 dark:hover:bg-green-400/15 text-green-800 border-green-200 dark:border-green-300/60 dark:text-green-400'
			}[state]
		)
	}
</script>

<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/aiStore'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink, WandSparkles } from 'lucide-svelte'
	import { getModifierKey } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		togglePanel,
		selected = false
	}: {
		togglePanel: () => void
		selected?: boolean
	} = $props()
</script>

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
		{@render button({ onPress: () => togglePanel() })}
	</DarkPopover>
{:else}
	<Popover placement="bottom" class="h-full">
		{#snippet trigger()}
			{@render button({ onPress: () => togglePanel() })}
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

{#snippet button({ onPress }: { onPress: () => void })}
	<Button
		size="xs"
		color="light"
		variant="default"
		onClick={onPress}
		startIcon={{ icon: WandSparkles }}
		iconOnly
		wrapperClasses="h-full"
		btnClasses={flowAIBtnClasses(selected ? 'selected' : 'default')}
	>
		AI Panel
	</Button>
{/snippet}
