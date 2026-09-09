<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { WandSparkles } from 'lucide-svelte'
	import { aiChatManager, type AIChatManager } from './chat/AIChatManager.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { getOpenInSessionHandoff } from '$lib/components/sessions/openInSessionContext'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { getContext } from 'svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	let {
		lang,
		error,
		jobId,
		moduleId
	}: {
		lang: SupportedLanguage
		/** The failing run's error, used when there is no job to point at. */
		error?: string
		/** The failing run's job id. Preferred over `error`: the chat reads the
		 * run itself with `get_job_logs`, which gives it the logs rather than
		 * just the thrown value, and keeps the composer readable. */
		jobId?: string
		/** Set when this sits in a flow step's preview, so the session opens on
		 * that step rather than the flow root. */
		moduleId?: string
	} = $props()

	// The enclosing editor's "Open in AI session" hand-off (ScriptEditor for a
	// standalone script, FlowBuilder for a step).
	const handoff = getOpenInSessionHandoff()
	const seedPrompt = $derived.by(() => {
		const what = moduleId ? `step \`${moduleId}\`` : 'this script'
		if (jobId) {
			return `The last test run of ${what} failed (job \`${jobId}\`). Read its logs, then fix the code.`
		}
		// No job to read: the error text has to travel with the request.
		return error
			? `Fix this error in ${what}:\n\n\`\`\`\n${error}\n\`\`\``
			: `Fix the error from the last run of ${what}.`
	})
	// Anonymous counter for a failing run being handed to AI, keyed by where the run was. All
	// three branches below report the same action: which one is on screen follows the session
	// gate and whether a chat is already beside this panel, not a choice made here.
	function logAiFix() {
		logFeatureUsage('ai_fix', 'requested', { key: moduleId ? 'flow_step' : 'script' })
	}

	const sessionSource = $derived.by(() => {
		const source = handoff?.source({ moduleId })
		if (!source) return undefined
		// The counter wraps the editor's own hook rather than replacing it: that hook persists
		// the draft the session opens on, so dropping it would fix an older copy of the code.
		const editorBeforeOpen = source.beforeOpen
		return {
			...source,
			seedPrompt,
			autoSend: true,
			beforeOpen: async () => {
				logAiFix()
				await editorBeforeOpen?.()
			}
		}
	})

	// Inside a session pane the chat is already beside this panel, so there is
	// nothing to hand off to: send into that chat instead. OpenInSessionButton
	// renders nothing there, which would otherwise leave the sessions population
	// with no fix affordance at all.
	const sessionScopedManager = getContext<AIChatManager | undefined>('aiChatManager')
</script>

{#if SUPPORTED_LANGUAGES.has(lang) && !$copilotInfo.workspaceDisabled}
	{#if sessionScopedManager}
		<Button
			title="Fix the failing run in this chat"
			size="xs"
			color="light"
			spacingSize="xs2"
			startIcon={{ icon: WandSparkles }}
			on:click={() => {
				logAiFix()
				sessionScopedManager.sendOrQueue(seedPrompt)
			}}
			btnClasses={AIBtnClasses('default')}
		>
			AI Fix
		</Button>
	{:else}
		<OpenInSessionButton
			source={sessionSource}
			btnClasses={AIBtnClasses('default')}
			label="Fix in AI session"
			tooltip="Open an AI session on this item and fix the failing run"
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
										logAiFix()
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
{/if}
