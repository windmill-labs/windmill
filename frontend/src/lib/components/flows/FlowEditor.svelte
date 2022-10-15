<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import { flowStore } from './flowStore'
	import { flowStateStore } from './flowState'
	import FlowPreviewButtons from './header/FlowPreviewButtons.svelte'

	export let initialPath: string
</script>

<div class="h-full overflow-hidden border-t">
	<Splitpanes>
		<Pane size={25} minSize={20} class="h-full flex flex-col">
			<FlowPreviewButtons />
			<div class="grow overflow-auto p-4 bg-gray-50">
				{#if $flowStore.value.modules && $flowStateStore.modules}
					<FlowModuleSchemaMap
						bind:modules={$flowStore.value.modules}
						bind:moduleStates={$flowStateStore.modules}
					/>
				{/if}
			</div>
		</Pane>
		<Pane size={75} minSize={40} class="!overflow-hidden">
			<FlowEditorPanel {initialPath} />
		</Pane>
	</Splitpanes>
</div>
