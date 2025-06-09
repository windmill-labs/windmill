<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import { classNames } from '$lib/utils'
	import { Bug, X } from 'lucide-svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { insertNewFailureModule } from '$lib/components/flows/flowStateUtils.svelte'
	import type { RawScript, ScriptLang } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'

	export let small: boolean

	const dispatch = createEventDispatcher<{
		generateStep: { moduleId: string; instructions: string; lang: ScriptLang }
	}>()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	const { selectedId, flowStateStore, flowStore } = flowEditorContext

	async function insertFailureModule(
		inlineScript?: {
			language: RawScript['language']
			subkind: 'pgsql' | 'flow'
			instructions?: string
		},
		wsScript?: { path: string; summary: string; hash: string | undefined }
	) {
		await insertNewFailureModule(flowStore, flowStateStore, inlineScript, wsScript)

		if (inlineScript?.instructions) {
			dispatch('generateStep', {
				moduleId: 'failure',
				instructions: inlineScript.instructions,
				lang: inlineScript.language
			})
		}

		$selectedId = 'failure'
		flowEditorContext.flowStore = flowStore
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->

<div
	id="flow-editor-error-handler"
	class={classNames(
		'z-10',
		'cursor-pointer border transition-colors duration-[400ms] ease-linear rounded-sm px-2 py-1 gap-2 bg-surface text-sm flex items-center flex-row',
		$selectedId?.includes('failure')
			? 'outline outline-offset-1 outline-2 outline-slate-900 dark:outline-slate-900/0 dark:bg-surface-secondary dark:border-gray-400'
			: ''
	)}
	style="min-width: {small ? '200px' : '230px'}; max-width: 275px;"
	on:click={() => {
		if (flowStore?.value?.failure_module) {
			$selectedId = 'failure'
		}
	}}
>
	<div class="flex items-center grow-0 min-w-0 gap-2">
		<Bug size={16} color={flowStore?.value?.failure_module ? '#3b82f6' : '#9CA3AF'} />
	</div>

	{#if !flowStore?.value?.failure_module}
		<div class="grow text-center font-bold text-xs">Error Handler</div>
	{:else}
		<div class="truncate grow min-w-0 text-center text-xs">
			{flowStore.value.failure_module?.summary ||
				(flowStore.value.failure_module?.value.type === 'rawscript'
					? `${flowStore.value.failure_module?.value.language}`
					: 'TBD')}
		</div>
	{/if}

	{#if !flowStore?.value?.failure_module}
		<InsertModuleButton
			index={0}
			placement={'top-center'}
			on:new={(e) => {
				insertFailureModule(e.detail.inlineScript)
			}}
			on:pickScript={(e) => {
				insertFailureModule(undefined, e.detail)
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
				flowStore.value.failure_module = undefined
				$selectedId = 'settings-metadata'
			}}
		>
			<X size={12} />
		</button>
	{/if}
</div>
