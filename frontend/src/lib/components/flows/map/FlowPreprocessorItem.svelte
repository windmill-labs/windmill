<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { FileCog } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { emptySchema } from '$lib/utils'
	import { NEVER_TESTED_THIS_FAR } from '../models'

	const { selectedId, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	function onToggle() {
		if ($flowStore?.value?.preprocessor_module) {
			$flowStore.value.preprocessor_module = undefined
			// By default, we return to settings when disabling the failure module
			$selectedId = 'settings-metadata'
		} else {
			const preprocessorModule = {
				schema: emptySchema(),
				previewResult: NEVER_TESTED_THIS_FAR
			}

			$flowStore.value.preprocessor_module = {
				id: 'preprocessor',
				value: { type: 'identity' }
			}
			$flowStateStore['preprocessor'] = preprocessorModule

			$selectedId = 'preprocessor'
			$flowStore = $flowStore
		}
	}
</script>

<button
	on:click={() => {
		if ($flowStore?.value?.preprocessor_module) {
			$selectedId = 'preprocessor'
		} else {
			onToggle()
		}
	}}
	class={twMerge(
		'border w-full rounded-sm p-2 bg-surface  text-sm cursor-pointer flex items-center',
		$selectedId == 'preprocessor'
			? 'outline outline-offset-1 outline-2  outline-slate-900 dark:bg-white/5 dark:outline-slate-800/60 dark:border-gray-400'
			: ''
	)}
>
	<FileCog size={16} />
	<span class="text-xs flex flex-row justify-between w-full gap-2 items-center truncate ml-1">
		Preprocessor
	</span>
	<Toggle
		size={'sm'}
		checked={Boolean($flowStore?.value?.preprocessor_module)}
		on:change={onToggle}
		id="preprocessor-toggle"
	/>
</button>
