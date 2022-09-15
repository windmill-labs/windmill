<script lang="ts">
	import { flowStore } from '$lib/components/flows/flowStore'
	import { faPen } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowPreviewButtons from './FlowPreviewButtons.svelte'
	import FlowStatus from './FlowStatus.svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowMenu from './FlowMenu.svelte'

	const { select } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="flex justify-between items-center border-y p-2 px-4">
	<div id="flow_title" class="flex justify-between items-center">
		<button on:click={() => select('settings')}>
			<span class="font-mono text-sm "> {$flowStore.path}</span>
			<Icon
				data={faPen}
				scale={0.8}
				class="text-gray-500 ml-2 flex justify-center items-center mb-0.5"
			/>
		</button>
	</div>
	<FlowStatus />
	<FlowMenu />
	<FlowPreviewButtons />
</div>
