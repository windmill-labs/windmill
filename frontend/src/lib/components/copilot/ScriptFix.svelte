<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { WandSparkles } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { getOpenInSessionHandoff } from '$lib/components/sessions/openInSessionContext'
	import { AIBtnClasses } from './chat/AIButtonStyle'

	let {
		lang,
		error,
		moduleId
	}: {
		lang: SupportedLanguage
		/** The failing run's error. A session cannot see the preview job, so the
		 * text has to travel with the hand-off. */
		error?: string
		/** Set when this sits in a flow step's preview, so the session opens on
		 * that step rather than the flow root. */
		moduleId?: string
	} = $props()

	// The enclosing editor's "Open in AI session" hand-off (ScriptEditor for a
	// standalone script, FlowBuilder for a step), seeded with the error so the
	// session lands with a ready-to-send prompt the user can still edit.
	const handoff = getOpenInSessionHandoff()
	const sessionSource = $derived.by(() => {
		const source = handoff?.source({ moduleId })
		if (!source) return undefined
		return {
			...source,
			seedPrompt: error
				? `Fix this error:\n\n\`\`\`\n${error}\n\`\`\``
				: 'Fix the error from the last run.'
		}
	})
</script>

{#if SUPPORTED_LANGUAGES.has(lang)}
	<OpenInSessionButton
		source={sessionSource}
		btnClasses={AIBtnClasses('default')}
		label="AI Fix"
		btnProps={{ iconOnly: false, startIcon: { icon: WandSparkles } }}
	>
		{#snippet fallback()}
			<Popover
				floatingConfig={{
					middleware: [
						autoPlacement({
							allowedPlacements: ['bottom-end', 'top-end']
						})
					]
				}}
				displayArrow={true}
			>
				{#snippet trigger()}
					<div class="flex flex-row">
						<Button
							title="Fix code"
							size="xs"
							color="light"
							spacingSize="xs2"
							startIcon={{ icon: WandSparkles }}
							on:click={() => {
								if ($copilotInfo.enabled) {
									aiChatManager.fix()
								}
							}}
							btnClasses="text-ai bg-violet-100 dark:bg-gray-700 min-w-[84px]"
							propagateEvent={!$copilotInfo.enabled}
						>
							AI Fix
						</Button>
					</div>
				{/snippet}
				{#snippet content()}
					<div class="p-4">
						<div class="w-80">
							<p class="text-sm"
								>Enable Windmill AI in the <a
									class="inline-flex flex-row items-center gap-1"
									href="{base}/workspace_settings?tab=ai"
									target="_blank">workspace settings</a
								></p
							></div
						>
					</div>
				{/snippet}
			</Popover>
		{/snippet}
	</OpenInSessionButton>
{/if}
