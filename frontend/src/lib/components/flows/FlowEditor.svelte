<!-- Flow Editor: Top level component -->
<script lang="ts">
	import { HSplitPane } from 'svelte-split-pane'
	import FlowEditorHeader from './header/FlowEditorHeader.svelte'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import { flowStateStore } from './flowState'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
</script>

<FlowEditorHeader />

<div class="h-full overflow-hidden">
	<HSplitPane leftPaneSize="30%" rightPaneSize="70%" minLeftPaneSize="20%" minRightPaneSize="20%">
		<left slot="left" class="h-full ">
			<div class="h-full overflow-auto p-4 bg-gray-50">
				<FlowModuleSchemaMap bind:flowModuleSchemas={$flowStateStore.modules} />
			</div>
		</left>
		<right slot="right" class="h-full">
			<div class="h-full overflow-auto bg-white ">
				<FlowEditorPanel />
			</div>
		</right>
	</HSplitPane>
</div>
