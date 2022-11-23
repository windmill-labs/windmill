<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faCalendarAlt, faSliders } from '@fortawesome/free-solid-svg-icons'
	import { classNames } from '$lib/utils'
	import { Badge } from '$lib/components/common'

	const { select, selectedId, schedule } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-sm p-2 bg-white border-gray-400 text-sm cursor-pointer flex items-center',
		$selectedId === 'settings' ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
	)
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click={() => select('settings')} class={settingsClass}>
	<Icon data={faSliders} class="mr-2" />
	<span
		class="text-xs font-bold flex flex-row justify-between w-full  gap-2 items-center flex-wrap truncate"
	>
		Settings
		<span
			class={classNames('badge', $schedule?.enabled ? 'badge-on' : 'badge-off')}
			on:click|stopPropagation={() => select('settings-schedule')}
		>
			{#if $schedule?.enabled}
				<Badge color="gray"
					>{$schedule.cron}
					<Icon class={$schedule.cron ? 'ml-2' : ''} data={faCalendarAlt} scale={0.8} />
				</Badge>
			{/if}
		</span>
	</span>
</div>
