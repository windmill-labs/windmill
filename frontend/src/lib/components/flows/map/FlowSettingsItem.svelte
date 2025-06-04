<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { classNames } from '$lib/utils'
	import { Badge } from '$lib/components/common'
	import { SlidersHorizontal } from 'lucide-svelte'

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let settingsClass = $derived(
		classNames(
			'border w-full rounded-sm p-2 bg-surface text-sm cursor-pointer flex items-center h-[32px]',
			$selectedId?.startsWith('settings')
				? 'border border-1  border-slate-800 dark:bg-white/5 dark:border-slate-400/60 dark:border-gray-400'
				: '',
			'hover:!bg-surface-secondary active:!bg-surface'
		)
	)
</script>

<button onclick={() => ($selectedId = 'settings-metadata')} class={settingsClass}>
	<SlidersHorizontal size={16} />
	<span
		class="text-xs font-bold flex flex-row justify-between w-full gap-2 items-center truncate ml-1"
	>
		<span>Settings</span>
		<span class="h-[18px] flex items-center">
			{#if $flowStore.value.same_worker}
				<Badge color="blue" baseClass="truncate">./shared</Badge>
			{/if}
		</span>
	</span>
</button>
