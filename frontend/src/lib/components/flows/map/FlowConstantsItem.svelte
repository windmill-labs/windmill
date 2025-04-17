<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { classNames } from '$lib/utils'
	import { DollarSign } from 'lucide-svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-sm p-2 bg-surface  text-sm cursor-pointer flex items-center',
		$selectedId == 'constants'
			? 'border border-1  border-slate-800 dark:bg-white/5 dark:border-slate-400/60 dark:border-gray-400'
			: '',
		'hover:!bg-surface-secondary active:!bg-surface'
	)
</script>

<button on:click={() => ($selectedId = 'constants')} class={settingsClass}>
	<DollarSign size={16} />
	<span class="text-xs flex flex-row justify-between w-full gap-2 items-center truncate ml-1">
		All Static Inputs
	</span>
</button>
