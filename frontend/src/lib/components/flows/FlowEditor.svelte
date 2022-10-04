<!-- Flow Editor: Top level component -->
<script lang="ts">
	import { HSplitPane } from 'svelte-split-pane'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import { flowStore } from './flowStore'
	import { flowStateStore } from './flowState'
	import FlowPreviewButtons from './header/FlowPreviewButtons.svelte'

	export let initialPath: string
</script>

<div class="h-full overflow-hidden border-t">
	<HSplitPane leftPaneSize="25%" rightPaneSize="75%" minLeftPaneSize="20%" minRightPaneSize="40%">
		<left slot="left" class="h-full flex flex-col ">
			<FlowPreviewButtons />
			<div class="grow overflow-auto p-4 bg-gray-50">
				{#if $flowStore.value.modules && $flowStateStore.modules}
					<FlowModuleSchemaMap
						bind:modules={$flowStore.value.modules}
						bind:moduleStates={$flowStateStore.modules}
					/>
				{/if}
			</div>
		</left>
		<right slot="right" class="h-full">
			<div class="h-full overflow-auto bg-white ">
				<FlowEditorPanel {initialPath} />
			</div>
		</right>
	</HSplitPane>
</div>
