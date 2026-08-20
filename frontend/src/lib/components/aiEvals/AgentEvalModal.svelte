<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { EvalsLocation } from './evalRuns'
	import EvalsPane from './EvalsPane.svelte'

	let {
		agentPath = undefined,
		open = $bindable(false),
		opWorkspace = undefined
	}: {
		/** The agent under test. A dataset and its runs belong to a saved agent, so without one
		 * the dialog can only say so. */
		agentPath?: string
		open?: boolean
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
	} = $props()

	// The dialog is what the breadcrumb's first segment names, so the path is composed here from
	// the dialog's own title and whatever level the pane reports being on.
	const TITLE = 'Evals'
	let location = $state<EvalsLocation | undefined>(undefined)
	let trail = $derived(
		location ? [{ label: TITLE, onclick: location.back }, { label: location.label }] : undefined
	)
	// Says what the dialog is for, so it belongs to the dialog's own name and goes when you
	// navigate past it: a level in is about one run, not about what runs are.
	let description = $derived(
		location
			? undefined
			: 'Each run answers a dataset of cases with this agent and scores the answers, so runs can be compared.'
	)
</script>

<!-- A dialog rather than a drawer: what it holds is a screen of its own — a history, then a run of
     it — read across its full width, not a panel beside the thing you came from. `fillHeight` so
     the table inside can size itself against the dialog instead of growing it. -->
<!-- `kind="X"` because there is nothing to cancel: what is inside is read and acted on, not filled
     in. The `sm:` widths are what actually win — the dialog's own are breakpoint-prefixed. -->
<Modal
	title={TITLE}
	{trail}
	{description}
	bind:open
	kind="X"
	class="w-[90vw] sm:w-[90vw] max-w-[1400px] sm:max-w-[1400px] h-[85vh]"
	fillHeight
>
	{#snippet titleBadge()}
		<!-- Beside the title rather than on the button that opens it: every way in lands here, so
		     this is the one place that says it once. -->
		<Badge color="blue" small class="shrink-0 !py-0 leading-4">Beta</Badge>
	{/snippet}
	<div class="h-full min-h-0">
		{#if agentPath}
			<EvalsPane {agentPath} {opWorkspace} bind:location />
		{:else}
			<div class="h-full flex flex-col items-center justify-center gap-2 p-6 text-center">
				<span class="text-sm text-emphasis">Evals run against a saved agent</span>
				<span class="text-xs text-secondary max-w-md">
					This agent is written into the flow step rather than saved as its own agent, so there is
					nothing for a dataset and its runs to belong to. Save it as a reusable agent from the
					step, and its evals start there.
				</span>
			</div>
		{/if}
	</div>
</Modal>
