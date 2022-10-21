<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowBranchWrapper from './FlowBranchWrapper.svelte'

	export let branchIndex: number
	export let branch: {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}
</script>

{#if branchIndex > 0}
	<FlowBranchWrapper bind:branch />
{:else}
	<div class="h-full flex flex-col">
		<FlowCard title="Branch">
			<div slot="header" class="grow">
				<input bind:value={branch.summary} placeholder={'Summary'} />
			</div>
		</FlowCard>
	</div>
{/if}
