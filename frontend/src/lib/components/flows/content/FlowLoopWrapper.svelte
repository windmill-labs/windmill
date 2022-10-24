<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	import { flowStore } from '../flowStore'

	import { selectedIdToIndexes } from '../utils'

	import FlowLoop from './FlowLoop.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: index = selectedIdToIndexes($selectedId)[0]
</script>

<FlowLoop bind:mod={$flowStore.value.modules[index]} {index} />
