<script lang="ts">
	import { classNames } from '$lib/utils'
	import {
		faArrowCircleDown,
		faArrowRotateBack,
		faArrowRotateForward,
		faCalendar,
		faCalendarAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'

	const { schedule, select } = getContext<FlowEditorContext>('FlowEditorContext')

	$: retriesEnabled = $flowStore.value.retry?.constant || $flowStore.value.retry?.exponential
</script>

<div class="flex space-x-1">
	<span
		class={classNames(
			' text-sm font-medium mr-2 px-2.5 py-0.5 rounded cursor-pointer flex items-center',
			$schedule?.enabled ? 'bg-blue-100 text-blue-800' : 'bg-gray-100 text-gray-800'
		)}
		on:click={() => select('settings-schedule')}
	>
		<Icon data={faCalendarAlt} class="mr-2" scale={0.8} />

		{$schedule?.enabled ? `Schedule: ${$schedule?.cron}` : 'Schedule disabled'}
	</span>
	<span
		class={classNames(
			' text-sm font-medium mr-2 px-2.5 py-0.5 rounded cursor-pointer flex items-center',
			retriesEnabled ? 'bg-blue-100 text-blue-800' : 'bg-gray-100 text-gray-800'
		)}
		on:click={() => select('settings-retries')}
	>
		<Icon data={faArrowRotateForward} class="mr-2" scale={0.8} />
		{retriesEnabled ? `Retries enabled` : 'Retries disabled'}
	</span>
</div>
