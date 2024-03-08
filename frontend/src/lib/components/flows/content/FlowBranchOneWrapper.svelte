<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'

	import BranchPredicateEditor from './BranchPredicateEditor.svelte'

	export let branch: {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
	export let parentModule: FlowModule
	export let previousModule: FlowModule | undefined
	export let noEditor: boolean
	export let enableAi = false
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Branch">
		<div slot="header" class="grow">
			<input bind:value={branch.summary} placeholder={'Summary'} />
		</div>
		<div class="overflow-hidden flex-grow">
			<h3 class="p-2">Predicate expression</h3>
			<BranchPredicateEditor
				{branch}
				{parentModule}
				{previousModule}
				{enableAi}
				on:updateSummary={(e) => {
					if (!branch.summary) {
						branch.summary = e.detail
					}
				}}
			/>
		</div>
	</FlowCard>
</div>
