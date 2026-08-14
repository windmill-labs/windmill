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
		/** The agent under test. Every entry point knows one. */
		agentPath?: string
		open?: boolean
		/** A case captured from a run or a conversation, opened for review before saving. */
		capture?: EvalCaseDraft
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
	} = $props()

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
			<EvalsPane
				{agentPath}
				{opWorkspace}
				capture={capture ? fromCaptureDraft(capture) : undefined}
			/>
		</div>
	</DrawerContent>
</Drawer>
