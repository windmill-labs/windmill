<script lang="ts">
	import type { Branches, FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import FlowBranchWrapper from './FlowBranchWrapper.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: [branchKeyword, parentIndex, branchIndex] = $selectedId.split('-')

	$: flowModule = $flowStore.value.modules[parentIndex]
	$: flowValue = flowModule.value as Branches
	$: branch = flowValue.branches[0] as {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
</script>

{#if Number(branchIndex) > 0}
	<FlowBranchWrapper />
{:else}
	<div class="h-full flex flex-col">
		<FlowCard title="Branch">
			<div slot="header" class="grow">
				<input bind:value={branch.summary} placeholder={'Summary'} />
			</div>
		</FlowCard>
	</div>
{/if}
