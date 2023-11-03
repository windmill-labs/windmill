<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'

	export let noEditor: boolean
	export let branch: {
		summary?: string
		skip_failure?: boolean
		modules: Array<FlowModule>
	}
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Branch">
		<div slot="header" class="grow">
			<input bind:value={branch.summary} placeholder={'Summary'} />
		</div>
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
