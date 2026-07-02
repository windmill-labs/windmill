<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/aiStore'
	import { aiUserDisabled } from '$lib/stores'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink, MessagesSquare } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	let {
		togglePanel,
		btnClasses,
		label = 'Open in AI session'
	}: {
		togglePanel: () => void
		btnClasses?: string
		/** Tooltip + accessible text of the icon-only button. */
		label?: string
	} = $props()
</script>

{#if $copilotInfo.enabled}
	<DarkPopover>
		{#snippet text()}
			{label}
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
				{#if $aiUserDisabled}
					<p class="text-sm">Windmill AI is disabled in your account settings.</p>
				{:else}
					<p class="text-sm"
						>Enable Windmill AI in the <a
							href="{base}/workspace_settings?tab=ai"
							target="_blank"
							class="inline-flex flex-row items-center gap-1"
							>workspace settings <ExternalLink size={16} /></a
						></p
					>
				{/if}
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
		startIcon={{ icon: MessagesSquare }}
		iconOnly
		{btnClasses}
	>
		{label}
	</Button>
{/snippet}
