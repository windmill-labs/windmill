<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faDollarSign } from '@fortawesome/free-solid-svg-icons'
	import { classNames } from '$lib/utils'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-sm p-2 bg-white border-gray-400 text-sm cursor-pointer flex items-center',
		$selectedId == 'constants' ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
	)
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click={() => select('constants')} class={settingsClass}>
	<Icon data={faDollarSign} class="mr-2" />
	<span class="text-xs flex flex-row justify-between w-full gap-2 items-center truncate">
		All Static Inputs
	</span>
</div>
