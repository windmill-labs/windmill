<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, Pencil } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	interface Props {
		/** Unset for a viewer who cannot edit the item: the card then offers no way to the editor. */
		onEditInstructions: (() => void) | undefined
		/** Empty renders the AI button alone, sized for the run form's action row, which
		 * is where the host places it; a prompt makes this a card of its own. */
		instructions: string
		runnableType: 'script' | 'flow'
		path: string | undefined
	}

	const { onEditInstructions, instructions, runnableType, path }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

	let expanded = $state(false)

	// Anonymous counter for this card being acted on, keyed by what it sits above. `beforeOpen` is
	// the hand-off's only per-click hook, and it runs before anything can turn the click away.
	function logAsked() {
		logFeatureUsage('run_form', 'ai_fill', { key: runnableType })
	}

	// The session's preview opens on this same deployed page (`/get/`), not the item's
	// editor: someone on a run page came to run the item, not to change it.
	const sessionSource = $derived(
		path
			? {
					page: () => `/${runnableType}s/get/${path}`,
					workspaceId: $operatingWorkspace ?? undefined,
					beforeOpen: logAsked
				}
			: undefined
	)
</script>

{#snippet aiButton(unifiedSize: 'sm' | 'md')}
	<OpenInSessionButton source={sessionSource} btnProps={{ unifiedSize }} deferToHeader />
{/snippet}

{#if $copilotInfo.workspaceDisabled}
	<!-- The workspace hid the assistant. -->
{:else if instructions}
	<div class="my-2 flex flex-col gap-1">
		<div class="flex flex-row gap-2 justify-between items-center">
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: expanded ? ChevronDown : ChevronRight }}
				onclick={() => (expanded = !expanded)}
			>
				AI prompt
			</Button>
			{@render aiButton('sm')}
		</div>
		{#if expanded}
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
{:else}
	{@render aiButton('md')}
{/if}
