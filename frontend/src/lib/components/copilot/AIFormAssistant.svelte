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
	import { pageHref } from '$lib/components/sessions/previewPaths'

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

	// The session's preview opens on this same deployed page (`/get/`), not the item's
	// editor: someone on a run page came to run the item, not to change it.
	const sessionSource = $derived(
		path
			? {
					page: () => pageHref(`/${runnableType}s/get/${path}`),
					workspaceId: $operatingWorkspace ?? undefined,
					beforeOpen: logAsked
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
			<!-- The fallback is a plain Button rather than AskAiButton, whose own session branch
			     would fire here too and open an empty session. -->
			<OpenInSessionButton source={sessionSource} btnProps={{ unifiedSize: 'sm' }}>
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
