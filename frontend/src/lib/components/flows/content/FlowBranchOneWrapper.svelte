<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'

	import BranchPredicateEditor from './BranchPredicateEditor.svelte'

	interface Props {
		branch: {
			summary?: string
			expr: string
			modules: Array<FlowModule>
		}
		parentModule: FlowModule
		previousModule: FlowModule | undefined
		noEditor: boolean
		enableAi?: boolean
	}

	let {
		branch = $bindable(),
		parentModule,
		previousModule,
		noEditor,
		enableAi = false
	}: Props = $props()
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Branch">
		{#snippet header()}
			<div class="grow">
				<input bind:value={branch.summary} placeholder={'Summary'} />
			</div>
		{/snippet}
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
