<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import type { EvalCaseDraft } from '$lib/gen'
	import EvalsPane from './EvalsPane.svelte'
	import { fromCaptureDraft } from './evalCaseUtils'

	let {
		agentPath = undefined,
		open = $bindable(),
		capture = undefined,
		opWorkspace = undefined
	}: {
		/** The agent under test. Absent when the drawer is opened from a capture, which names the
		 * agent it ran against. */
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

	let drawer: Drawer | undefined = $state()

	// The drawer is opened by a bound flag from several entry points, so opening is driven from
	// the prop rather than from a method the caller would have to reach for.
	$effect(() => {
		if (open) {
			drawer?.openDrawer()
		} else {
			drawer?.closeDrawer()
		}
	})
</script>

<Drawer bind:this={drawer} size="1400px" on:close={() => (open = false)}>
	<DrawerContent
		title="Evals"
		tooltip="Run this agent against a dataset of cases and score the answers."
		noPadding
		on:close={() => (open = false)}
	>
		<div class="h-full">
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
						there is nothing for a dataset and its runs to belong to. Save it as a reusable agent
						from the step, and its evals start there.
					</span>
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
