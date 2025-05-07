<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import { classNames, emptySchema } from '$lib/utils'
	import type { FlowModuleState } from '../flowState'
	import { NEVER_TESTED_THIS_FAR } from '../models'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { fade } from 'svelte/transition'
	import { Bug, X } from 'lucide-svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { createInlineScriptModule, pickScript } from '$lib/components/flows/flowStateUtils'
	import type { FlowModule, RawScript } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'

	export let small: boolean

	const { selectedId, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewFailureModule(
		inlineScript?: {
			language: RawScript['language']
			subkind: 'pgsql' | 'flow'
		},
		wsScript?: { path: string; summary: string; hash: string | undefined }
	) {
		var module: FlowModule = {
			id: 'failure',
			value: { type: 'identity' }
		}
		var state: FlowModuleState = {
			schema: emptySchema(),
			previewResult: NEVER_TESTED_THIS_FAR
		}

		if (inlineScript) {
			;[module, state] = await createInlineScriptModule(
				inlineScript.language,
				'failure',
				inlineScript.subkind,
				'failure'
			)
		} else if (wsScript) {
			;[module, state] = await pickScript(wsScript.path, wsScript.summary, module.id, wsScript.hash)
		}

		$flowStore.value.failure_module = module
		$flowStateStore[module.id] = state

		$selectedId = 'failure'
		$flowStore = $flowStore
	}

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->

<div
	id="flow-editor-error-handler"
	class={classNames(
		'z-10',
		$copilotCurrentStepStore !== undefined ? 'border-gray-500/75' : 'cursor-pointer',
		'border transition-colors duration-[400ms] ease-linear rounded-sm px-2 py-1 gap-2 bg-surface text-sm flex items-center flex-row',
		$selectedId?.includes('failure')
			? 'outline outline-offset-1 outline-2 outline-slate-900 dark:outline-slate-900/0 dark:bg-surface-secondary dark:border-gray-400'
			: ''
	)}
	style="min-width: {small ? '200px' : '230px'}; max-width: 275px;"
	on:click={() => {
		if ($copilotCurrentStepStore !== undefined) return
		if ($flowStore?.value?.failure_module) {
			$selectedId = 'failure'
		}
	}}
>
	{#if $copilotCurrentStepStore !== undefined}
		<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900]"></div>
	{/if}
	<div class="flex items-center grow-0 min-w-0 gap-2">
		<Bug size={16} color={$flowStore?.value?.failure_module ? '#3b82f6' : '#9CA3AF'} />
	</div>

	{#if !$flowStore?.value?.failure_module}
		<div class="grow text-center font-bold text-xs">Error Handler</div>
	{:else}
		<div class="truncate grow min-w-0 text-center text-xs">
			{$flowStore.value.failure_module?.summary ||
				($flowStore.value.failure_module?.value.type === 'rawscript'
					? `${$flowStore.value.failure_module?.value.language}`
					: 'TBD')}
		</div>
	{/if}

	{#if !$flowStore?.value?.failure_module}
		<InsertModuleButton
			disableAi={false}
			index={0}
			placement={'top-center'}
			on:new={(e) => {
				insertNewFailureModule(e.detail.inlineScript)
			}}
			on:pickScript={(e) => {
				insertNewFailureModule(undefined, e.detail)
			}}
			kind="failure"
		/>
	{:else}
		<button
			title="Delete failure script"
			type="button"
			class={twMerge(
				'w-5 h-5 flex items-center justify-center grow-0 shrink-0',
				'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
				'text-secondary',
				'bg-surface focus:outline-none hover:bg-surface-hover rounded '
			)}
			on:click={() => {
				$flowStore.value.failure_module = undefined
				$selectedId = 'settings-metadata'
			}}
		>
			<X size={12} />
		</button>
	{/if}
</div>
