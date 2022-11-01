<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import { flowStore } from './flowStore'

	export let initialPath: string
</script>

<div class="h-full overflow-hidden border-t">
	<Splitpanes>
		<Pane size={25} minSize={20} class="h-full">
			<div class="grow overflow-auto p-4 bg-gray h-full bg-gray-50 relative">
				{#if $flowStore.value.modules}
					<FlowModuleSchemaMap bind:modules={$flowStore.value.modules} root />
				{/if}
			</div>
		</Pane>
		<Pane size={75} minSize={40}>
			<FlowEditorPanel {initialPath} />
		</Pane>
	</Splitpanes>
</div>
