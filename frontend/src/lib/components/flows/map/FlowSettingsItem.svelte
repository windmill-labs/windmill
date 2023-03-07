<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { classNames } from '$lib/utils'
	import { Badge } from '$lib/components/common'
	import { SlidersHorizontal } from 'lucide-svelte'

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	$: settingsClass = classNames(
		'border w-full rounded-sm p-2 bg-white border-gray-400 text-sm cursor-pointer flex items-center',
		$selectedId?.startsWith('settings')
			? 'outline outline-offset-1 outline-2  outline-slate-900'
			: ''
	)
</script>

<button on:click={() => ($selectedId = 'settings-metadata')} class={settingsClass}>
	<SlidersHorizontal size={16} />
	<span
		class="text-xs font-bold flex flex-row justify-between w-full gap-2 items-center truncate ml-1"
	>
		Settings

		{#if $flowStore.value.same_worker}
			<Badge color="blue" baseClass="truncate">./shared</Badge>
		{/if}
	</span>
</button>
