<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const { schedule, select } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="flex space-x-1">
	<span
		class={classNames(
			' text-sm font-medium mr-2 px-2.5 py-0.5 rounded cursor-pointer',
			$schedule?.enabled ? 'bg-sky-100 text-sky-800' : 'bg-gray-100 text-gray-800'
		)}
		on:click={() => select('settings-schedule')}
	>
		{$schedule?.enabled ? `Schedule: ${$schedule?.cron}` : 'Schedule disabled'}
	</span>
</div>
