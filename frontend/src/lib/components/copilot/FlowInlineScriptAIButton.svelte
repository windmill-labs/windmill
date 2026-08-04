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
	import { tick, type ComponentProps } from 'svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { getOpenInSessionHandoff } from '$lib/components/sessions/openInSessionContext'

	interface Props {
		moduleId?: string
		/** Materializes Monaco's in-flight keystrokes into the draft. This button
		 * sits in the code editor's own toolbar, so "type, then click" is the
		 * normal case, and the session preview loads the item from its draft —
		 * without this the last (sub-second) edits would not be in it. */
		flushEditor?: () => void
		btnProps?: ComponentProps<typeof Button>
	}

	const { moduleId, flushEditor, btnProps }: Props = $props()

	// The enclosing editor's "Open in AI session" hand-off, opening the preview on
	// the step this toolbar edits.
	const handoff = getOpenInSessionHandoff()
	const sessionSource = $derived.by(() => {
		const source = handoff?.source({ moduleId })
		if (!source || !flushEditor) return source
		return {
			...source,
			beforeOpen: async () => {
				flushEditor()
				// The flush lands in the draft store through an effect; let it run
				// before the hand-off persists that store.
				await tick()
				await source.beforeOpen?.()
			}
		}
	})

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
