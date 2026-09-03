<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Pencil, WandSparkles } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { workspaceStore } from '$lib/stores'
	import { copilotInfo } from '$lib/aiStore'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	interface Props {
		onEditInstructions: () => void
		instructions: string
		runnableType: 'script' | 'flow'
		path: string | undefined
	}

	const { onEditInstructions, instructions, runnableType, path }: Props = $props()

	// Anonymous counter for this card being acted on, keyed by what it sits above. The two
	// branches share one counter: they are the same intent, and which of them is on screen
	// follows the user's session gate rather than a choice made here. `beforeOpen` is the
	// hand-off's only per-click hook, and it runs before anything can turn the click away.
	function logAsked() {
		logFeatureUsage('run_form', 'ai_fill', { key: runnableType })
	}

	async function fillFormWithAI() {
		logAsked()
		aiChatManager.openChat()
		aiChatManager.askAi(`Analyze the ${runnableType} form on this page and fill the inputs for me`)
	}

	// A session cannot reach this page's form (the preview is a separate editor,
	// and form filling drives the DOM through NAVIGATOR mode), so the hand-off
	// asks it to run the item instead. Naming the DEPLOYED version matters: the
	// test_run_* tools prefer drafts, which is not what this page runs.
	const sessionSource = $derived(
		path
			? {
					target: { kind: runnableType, path } as const,
					workspaceId: $workspaceStore ?? undefined,
					beforeOpen: logAsked,
					seedPrompt:
						`Run the deployed ${runnableType} \`${path}\` for me. Pick sensible inputs, ` +
						`tell me what you chose, then run it.` +
						(instructions ? `\n\nHow to choose the inputs:\n${instructions}` : '')
				}
			: undefined
	)
</script>

{#if !$copilotInfo.workspaceDisabled}
	<div class="my-3 p-3 bg-surface-secondary rounded-md relative flex flex-col gap-3">
		<div class="flex flex-row gap-2 justify-between items-center">
			<!-- Heading stays neutral because the two branches do different things: the
		     hand-off runs the item, the legacy path fills the form. Each button
		     names its own action. A plain Button rather than AskAiButton, whose own
		     session branch would fire here too and open an empty session. -->
			<h3 class="text-sm font-medium">AI can help with these inputs</h3>
			<OpenInSessionButton
				source={sessionSource}
				label="Run in AI session"
				tooltip="Open an AI session that picks inputs and runs this"
				btnProps={{ iconOnly: false, startIcon: { icon: WandSparkles } }}
			>
				{#snippet fallback()}
					<Button
						unifiedSize="md"
						startIcon={{ icon: WandSparkles }}
						btnClasses={AIBtnClasses('default')}
						on:click={fillFormWithAI}
					>
						Fill with AI
					</Button>
				{/snippet}
			</OpenInSessionButton>
		</div>
		<div class="flex flex-row gap-2 items-center">
			<p class="text-sm text-primary">
				{instructions
					? 'Instructions: ' + instructions
					: 'No AI instructions provided. Click edit to add guidance for AI form filling.'}
			</p>
			<Button
				color="light"
				size="xs2"
				startIcon={{
					icon: Pencil
				}}
				iconOnly
				on:click={onEditInstructions}
			/>
		</div>
	</div>
{/if}
