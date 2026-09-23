<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/aiStore'
	import { aiUserDisabled } from '$lib/stores'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink, MessagesSquare } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { ComponentProps } from 'svelte'

	let {
		togglePanel,
		btnClasses,
		btnProps,
		label = 'Open in AI session',
		tooltip
	}: {
		togglePanel: () => void
		btnClasses?: string
		/** Overrides for the host's button styling (an editor toolbar sizes and
		 * flattens it to match its neighbours). `btnClasses` still wins. */
		btnProps?: ComponentProps<typeof Button>
		/** Tooltip + accessible text of the icon-only button. */
		label?: string
		/** Hover text, when the label alone doesn't say where the button leads.
		 * A host that renamed the button ("AI Fix") uses this to keep "in a new
		 * AI session" discoverable. Defaults to `label`. */
		tooltip?: string
	} = $props()

	const hoverText = $derived(tooltip ?? label)
</script>

{#if $copilotInfo.enabled}
	<DarkPopover>
		{#snippet text()}
			{hoverText}
		{/snippet}
		{@render button({ onPress: () => togglePanel() })}
	</DarkPopover>
{:else if !$copilotInfo.workspaceDisabled}
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
		{...btnProps}
		{btnClasses}
	>
		{label}
	</Button>
{/snippet}
