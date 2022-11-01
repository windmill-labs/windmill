<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faCalendarAlt, faSliders } from '@fortawesome/free-solid-svg-icons'
	import { classNames } from '$lib/utils'

	const { select, selectedId, schedule } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4 sticky top-0 z-10',
		$selectedId === 'settings' ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
	)
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click={() => select('settings')} class={settingsClass}>
	<Icon data={faSliders} class="mr-2" />
	<span class="text-xs font-bold flex flex-row justify-between w-full flex-wrap gap-2 items-center">
		Settings
		<span
			class={classNames('badge', $schedule?.enabled ? 'badge-on' : 'badge-off')}
			on:click|stopPropagation={() => select('settings-schedule')}
		>
			{$schedule.cron}
			<Icon class={$schedule.cron ? 'ml-2' : ''} data={faCalendarAlt} scale={0.8} />
		</span>
	</span>
</div>
