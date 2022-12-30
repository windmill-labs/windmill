<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faSliders } from '@fortawesome/free-solid-svg-icons'
	import { classNames } from '$lib/utils'
	import { Badge } from '$lib/components/common'
	import { flowStore } from '../flowStore'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-sm p-2 bg-white border-gray-400 text-sm cursor-pointer flex items-center',
		$selectedId?.startsWith('settings')
			? 'outline outline-offset-1 outline-2  outline-slate-900'
			: ''
	)
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click={() => select('settings-metadata')} class={settingsClass}>
	<Icon data={faSliders} class="mr-2" />
	<span class="text-xs font-bold flex flex-row justify-between w-full gap-2 items-center truncate">
		Settings

		{#if $flowStore.value.same_worker}
			<Badge color="blue" baseClass="truncate">./shared</Badge>
		{/if}
	</span>
</div>
