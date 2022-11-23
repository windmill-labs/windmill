<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faBug } from '@fortawesome/free-solid-svg-icons'
	import { classNames, emptySchema } from '$lib/utils'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import Toggle from '$lib/components/Toggle.svelte'
	import { flowStore } from '../flowStore'
	import { NEVER_TESTED_THIS_FAR } from '../utils'

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function onToggle() {
		if ($flowStore.value.failure_module) {
			$flowStore.value.failure_module = undefined
			// By default, we return to settings when disabling the failure module
			select('settings')
		} else {
			const failureModule: FlowModuleState = {
				schema: emptySchema(),
				previewResult: NEVER_TESTED_THIS_FAR
			}
			$flowStateStore['failure'] = failureModule
			$flowStore.value.failure_module = {
				id: 'failure',
				value: { type: 'identity' }
			}
			select('failure')
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	on:click={() => {
		if ($flowStore.value.failure_module) {
			select('failure')
		} else {
			onToggle()
		}
	}}
	class={classNames(
		'border rounded-sm px-2 py-1 bg-white text-sm border-gray-400 cursor-pointer flex flex-col overflow-x-hidden ',
		$selectedId.includes('failure') ? 'outline outline-offset-1 outline-2 outline-slate-900' : ''
	)}
>
	<div class=" flex justify-between items-center flex-wrap">
		<div>
			<Icon data={faBug} class="mr-2" />
			<span class="font-bold text-xs">Error handler</span>
		</div>
		<div class="-my-1">
			<Toggle checked={Boolean($flowStore.value.failure_module)} on:change={onToggle} />
		</div>
	</div>

	<div class="w-full truncate block">
		{#if Boolean($flowStore.value.failure_module)}
			<span>
				{$flowStore.value.failure_module?.summary ||
					($flowStore.value.failure_module?.value.type === 'rawscript'
						? `Inline ${$flowStore.value.failure_module?.value.language}`
						: 'Select a script')}
			</span>
		{/if}
	</div>
</div>
