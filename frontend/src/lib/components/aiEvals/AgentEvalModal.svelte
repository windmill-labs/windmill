<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import type { EvalCaseDraft } from '$lib/gen'
	import EvalsPane from './EvalsPane.svelte'
	import { fromCaptureDraft } from './evalCaseUtils'

	let {
		agentPath = undefined,
		open = $bindable(false),
		capture = undefined,
		opWorkspace = undefined
	}: {
		/** The agent under test. Absent when opened from a capture, which names the agent it ran
		 * against. */
		agentPath?: string
		open?: boolean
		/** A case captured from a run or a conversation, opened for review before saving. */
		capture?: EvalCaseDraft
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
	} = $props()

	// A capture names the agent it ran against, which is the agent whose dataset it belongs in.
	let path = $derived(agentPath ?? capture?.agent_path)
</script>

<!-- A dialog rather than a drawer: what it holds is a screen of its own — a history, then a run of
     it — read across its full width, not a panel beside the thing you came from. `fillHeight` so
     the table inside can size itself against the dialog instead of growing it. -->
<!-- `kind="X"` because there is nothing to cancel: what is inside is read and acted on, not filled
     in. The `sm:` widths are what actually win — the dialog's own are breakpoint-prefixed. -->
<Modal
	title="Evals"
	bind:open
	kind="X"
	class="w-[90vw] sm:w-[90vw] max-w-[1400px] sm:max-w-[1400px] h-[85vh]"
	fillHeight
>
	<div class="h-full min-h-0">
		{#if path}
			<EvalsPane
				agentPath={path}
				{opWorkspace}
				capture={capture ? fromCaptureDraft(capture) : undefined}
			/>
		{:else}
			<div class="h-full flex flex-col items-center justify-center gap-2 p-6 text-center">
				<span class="text-sm text-emphasis">Evals run against a saved agent</span>
				<span class="text-xs text-secondary max-w-md">
					This run's agent is written into the flow step rather than saved as its own agent, so
					there is nothing for a dataset and its runs to belong to. Save it as a reusable agent from
					the step, and its evals start there.
				</span>
			</div>
		{/if}
	</div>
</Modal>
