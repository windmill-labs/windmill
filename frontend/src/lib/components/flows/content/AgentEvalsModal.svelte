<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import EvalsPane from '$lib/components/aiEvals/EvalsPane.svelte'
	import type { EvalsLocation } from '$lib/components/aiEvals/evalUtils'
	import type { AgentDraft } from '$lib/gen'

	/** An agent's evals in a dialog of their own, over the page that opened them. */
	interface Props {
		agentPath: string
		workspace: string
		/** The unsaved edits, offered to a run as an alternative to the deployed version. */
		editedConfig?: () => AgentDraft
	}

	let { agentPath, workspace, editedConfig = undefined }: Props = $props()

	let open = $state(false)
	// Where the evals pane is within itself, so its levels extend the dialog's trail. Cleared on the
	// way in: the pane reports a level once it is on one, and never that it is back at its root.
	let location = $state<EvalsLocation | undefined>(undefined)

	export function openModal() {
		location = undefined
		open = true
	}
</script>

{#if open}
	<Modal
		bind:open
		kind="X"
		fillHeight
		enterConfirms={false}
		title="Evals"
		trail={[
			{ label: 'Evals', onclick: location ? location.back : undefined },
			...(location ? [{ label: location.label }] : [])
		]}
		class="w-[92vw] sm:w-[92vw] max-w-[1500px] sm:max-w-[1500px] h-[88vh]"
	>
		{#snippet titleBadge()}
			{#if !location}
				<Badge color="blue" small class="shrink-0 !py-0 leading-4">Beta</Badge>
			{/if}
		{/snippet}
		<EvalsPane {agentPath} opWorkspace={workspace} {editedConfig} bind:location active={open} />
	</Modal>
{/if}
