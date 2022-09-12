<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: upToDisabled = ['settings', 'inputs', 'schedules'].includes($selectedId)
</script>

<div class="flex space-x-2">
	<button
		disabled={upToDisabled}
		type="button"
		class="py-2 px-3 text-xs font-medium text-center text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
	>
		Up this this step
	</button>
	<button
		type="button"
		class="py-2 px-3 text-xs font-medium text-center text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
	>
		Preview whole
	</button>
</div>
