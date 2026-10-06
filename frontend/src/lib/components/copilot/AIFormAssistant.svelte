<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, Pencil, WandSparkles } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	interface Props {
		/** Unset for a viewer who cannot edit the item: the card then offers no way to the editor. */
		onEditInstructions: (() => void) | undefined
		instructions: string
		runnableType: 'script' | 'flow'
		path: string | undefined
	}

	const { onEditInstructions, instructions, runnableType, path }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

	let expanded = $state(false)

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
					workspaceId: $operatingWorkspace ?? undefined,
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
	<div class="my-2 flex flex-col gap-1">
		<div class="flex flex-row gap-2 justify-between items-center">
			{#if instructions}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: expanded ? ChevronDown : ChevronRight }}
					onclick={() => (expanded = !expanded)}
				>
					Additional prompt for AI
				</Button>
			{:else}
				<span></span>
			{/if}
			<!-- Each button names its own action because the two branches do different things:
			     the hand-off runs the item, the legacy path fills the form. A plain Button rather
			     than AskAiButton, whose own session branch would fire here too and open an empty
			     session. -->
			<OpenInSessionButton
				source={sessionSource}
				label="Run in AI session"
				tooltip="Open an AI session that picks inputs and runs this"
				btnProps={{ iconOnly: false, unifiedSize: 'sm', startIcon: { icon: WandSparkles } }}
			>
				{#snippet fallback()}
					<Button
						unifiedSize="sm"
						startIcon={{ icon: WandSparkles }}
						btnClasses={AIBtnClasses('default')}
						onclick={fillFormWithAI}
					>
						Fill with AI
					</Button>
				{/snippet}
			</OpenInSessionButton>
		</div>
		{#if instructions && expanded}
			<div
				transition:slide={{ duration: 120 }}
				class="flex flex-row gap-2 items-start justify-between p-2 bg-surface-secondary rounded-md"
			>
				<p class="text-xs text-primary whitespace-pre-wrap">{instructions}</p>
				{#if onEditInstructions}
					<Button
						variant="subtle"
						unifiedSize="xs"
						startIcon={{ icon: Pencil }}
						iconOnly
						onclick={onEditInstructions}
					/>
				{/if}
			</div>
		{/if}
	</div>
{/if}
