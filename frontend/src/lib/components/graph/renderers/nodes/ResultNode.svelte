<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { ResultN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'

	interface Props {
		data: ResultN['data']
	}

	let { data }: Props = $props()

	const { selectionManager } = getGraphContext()
</script>

<NodeWrapper enableSourceHandle={false}>
	{#snippet children({ darkMode })}
		<VirtualItem
			id={'Result'}
			label={'Result'}
			selectable={true}
			selected={selectionManager?.getSelectedId() === 'Result'}
			hideId={true}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
			nodeKind="result"
			editMode={data.editMode}
			job={data.job}
			showJobStatus={data.showJobStatus}
		/>
	{/snippet}
</NodeWrapper>
