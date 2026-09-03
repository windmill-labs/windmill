<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { AgentDraft } from '$lib/gen'
	import type { EvalsLocation } from './evalUtils'
	import EvalsPane from './EvalsPane.svelte'

	let {
		agentPath = undefined,
		open = $bindable(),
		opWorkspace = undefined,
		editedConfig = undefined
	}: {
		/** The agent under test. A dataset and its runs belong to a saved agent. */
		agentPath?: string
		open: boolean
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** Opened from an agent being edited: the edits, as the step holds them, are what a run is
		 * offered on. Everywhere else the agent is what is deployed. */
		editedConfig?: () => AgentDraft
	} = $props()

	const TITLE = 'Evals'
	let location = $state<EvalsLocation | undefined>(undefined)
	let trail = $derived(
		location ? [{ label: TITLE, onclick: location.back }, { label: location.label }] : undefined
	)
</script>

<!-- The `sm:` widths are what actually win: the dialog's own are breakpoint-prefixed. -->
<Modal
	title={TITLE}
	{trail}
	paginated
	enterConfirms={false}
	bind:open
	kind="X"
	class="w-[90vw] sm:w-[90vw] max-w-[1400px] sm:max-w-[1400px] h-[85vh]"
	fillHeight
>
	{#snippet titleBadge()}
		<Badge color="blue" small class="shrink-0 !py-0 leading-4">Beta</Badge>
	{/snippet}
	<div class="h-full min-h-0">
		{#if agentPath}
			<!-- Keyed: the resources page keeps one dialog for every row, so a different agent must
			     start from nothing rather than inherit the runs and the open run of the last one. -->
			{#key `${opWorkspace ?? ''}:${agentPath}`}
				<EvalsPane {agentPath} {opWorkspace} {editedConfig} bind:location />
			{/key}
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
