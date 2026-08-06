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

	let { branch, parentModule, previousModule, noEditor, enableAi = false }: Props = $props()
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Branch">
		{#snippet header()}
			<div class="grow">
				<input bind:value={branch.summary} placeholder={'Summary'} />
			</div>
		{/snippet}
		<div class="flex h-full min-h-0 flex-col overflow-auto p-4" style="scrollbar-gutter: stable">
			<BranchPredicateEditor {branch} {parentModule} {previousModule} {enableAi} />
		</div>
	</FlowCard>
</div>
