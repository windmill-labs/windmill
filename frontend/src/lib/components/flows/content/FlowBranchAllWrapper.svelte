<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'

	interface Props {
		noEditor: boolean
		branch: {
			summary?: string
			skip_failure?: boolean
			modules: Array<FlowModule>
		}
	}

	let { noEditor, branch = $bindable() }: Props = $props()
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Branch">
		{#snippet header()}
			<div class="grow">
				<input bind:value={branch.summary} placeholder={'Summary'} />
			</div>
		{/snippet}
		<div class="p-4">
			<div class="mt-2 mb-2 text-sm font-bold">Skip failures</div>
			<Toggle
				bind:checked={branch.skip_failure}
				options={{
					right: 'Skip failures'
				}}
			/>
		</div>
	</FlowCard>
</div>
