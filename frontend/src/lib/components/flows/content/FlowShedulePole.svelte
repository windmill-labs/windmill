<script lang="ts">
	import SchedulePollPanel from '$lib/components/details/SchedulePollPanel.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="p-4">
	<SchedulePollPanel isFlow={true} path={$pathStore ?? ''} />
</div>
