<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowLoop from './FlowLoop.svelte'
	import { flowModuleMap } from '../flowModuleMap'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	// Pointer
	$: mod = $flowModuleMap[$selectedId].flowModule

	const index = 0
</script>

<FlowLoop bind:mod {index} />
