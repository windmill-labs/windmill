<script lang="ts">
	import { copilotInfo } from '$lib/stores'
	import { autoPlacement } from '@floating-ui/core'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '../common/button/Button.svelte'
	import { ExternalLink, WandSparkles } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { twMerge } from 'tailwind-merge'
	import { aiChatManager, AIMode } from './chat/AIChatManager.svelte'

	const aiChatScriptModeClasses = $derived(
		aiChatManager.mode === AIMode.SCRIPT && aiChatManager.isOpen
			? 'dark:bg-violet-900 bg-violet-100'
			: ''
	)
</script>

{#snippet button(onClick?: () => void)}
	<Button
		size="xs"
		color="light"
		btnClasses={twMerge('!px-2', aiChatScriptModeClasses)}
		{onClick}
		iconOnly
		title="Open AI chat in script mode"
		startIcon={{ icon: WandSparkles, classes: 'text-violet-800 dark:text-violet-400' }}
	/>
{/snippet}

{#if $copilotInfo.enabled}
	{@render button(() => {
		aiChatManager.openChat()
		aiChatManager.changeMode(AIMode.SCRIPT)
	})}
{:else}
	<Popover
		floatingConfig={{
			middleware: [
				autoPlacement({
					allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
				})
			]
		}}
	>
		{#snippet trigger()}
			{@render button()}
		{/snippet}
		{#snippet content({ close })}
			<div class="p-4">
				<p class="text-sm">
					Enable Windmill AI in the <a
						href="{base}/workspace_settings?tab=ai"
						target="_blank"
						class="inline-flex flex-row items-center gap-1"
					>
						workspace settings <ExternalLink size={16} />
					</a>
				</p>
			</div>
		{/snippet}
	</Popover>
{/if}
