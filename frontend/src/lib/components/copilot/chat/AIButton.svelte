<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/aiStore'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink, WandSparkles } from 'lucide-svelte'
	import { getModifierKey } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'

	let {
		togglePanel,
		btnClasses
	}: {
		togglePanel: () => void
		btnClasses?: string
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
		unifiedSize="sm"
		color="light"
		variant="default"
		onClick={onPress}
		startIcon={{ icon: WandSparkles }}
		iconOnly
		{btnClasses}
	>
		AI Panel
	</Button>
{/snippet}
