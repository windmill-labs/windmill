<script lang="ts">
	import { flowStore } from '$lib/components/flows/flowStore'
	import { faPen } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowPreviewButtons from './FlowPreviewButtons.svelte'
	import FlowStatus from './FlowStatus.svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const { schedule, select } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="flex justify-between items-center border-y p-2 px-4">
	<div id="flow_title">
		{$flowStore.path}
		<button on:click={() => select('settings')}><Icon data={faPen} /></button>
	</div>
	<FlowStatus />
	<FlowPreviewButtons />
</div>
