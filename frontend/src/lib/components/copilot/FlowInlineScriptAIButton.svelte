<script lang="ts">
	import { autoPlacement } from '@floating-ui/core'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '../common/button/Button.svelte'
	import { ExternalLink, WandSparkles } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { twMerge } from 'tailwind-merge'
	import { aiChatManager, AIMode } from './chat/AIChatManager.svelte'
	import { chatState } from './chat/sharedChatState.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import type { ComponentProps } from 'svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { getOpenInSessionHandoff } from '$lib/components/sessions/openInSessionContext'

	interface Props {
		moduleId?: string
		btnProps?: ComponentProps<typeof Button>
	}

	const { moduleId, btnProps }: Props = $props()

	// The enclosing editor's "Open in AI session" hand-off, opening the preview on
	// the step this toolbar edits.
	const handoff = getOpenInSessionHandoff()
	const sessionSource = $derived(handoff?.source({ moduleId }))

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
		title="Open AI chat"
		startIcon={{ icon: WandSparkles, classes: 'text-ai' }}
		{...btnProps}
	/>
{/snippet}

<OpenInSessionButton source={sessionSource} {btnProps}>
	{#snippet fallback()}
		<!-- Legacy docked chat: this button only opens that pane, so without one there
		     is nothing to open and it hides rather than rendering a dead click. -->
		{#if chatState.dockedChatAvailable}
			{#if $copilotInfo.enabled}
				{@render button(() => {
					aiChatManager.openChat()
					const availableContext = aiChatManager.contextManager.getAvailableContext()
					aiChatManager.contextManager.setSelectedModuleContext(moduleId, availableContext)
				})}
			{:else}
				<Popover
					floatingConfig={{
						middleware: [
							autoPlacement({
								allowedPlacements: [
									'bottom-start',
									'bottom-end',
									'top-start',
									'top-end',
									'top',
									'bottom'
								]
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
		{/if}
	{/snippet}
</OpenInSessionButton>
