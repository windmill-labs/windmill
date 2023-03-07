<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './types'

	export let initialPath: string
	export let loading: boolean

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let size = 50
</script>

<div class="h-full overflow-hidden border-t">
	<Splitpanes>
		<Pane {size} minSize={15} class="h-full">
			<div class="grow overflow-auto bg-gray h-full bg-gray-50 relative">
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if $flowStore.value.modules}
					<FlowModuleSchemaMap bind:modules={$flowStore.value.modules} />
				{/if}
			</div>
		</Pane>
		<Pane class="drop-shadow-2xl" size={100 - size} minSize={40}>
			{#if loading}
				<div class="w-full h-full ">
					<div class="block m-auto mt-40 w-10">
						<WindmillIcon class="animate-[spin_6s_linear_infinite]" height="40px" width="40px" />
					</div>
				</div>
			{:else}
				<FlowEditorPanel {initialPath} />
			{/if}
		</Pane>
	</Splitpanes>
</div>
