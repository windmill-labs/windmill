<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import { classNames } from '$lib/utils'
	import { Bug, X } from 'lucide-svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { insertNewFailureModule } from '$lib/components/flows/flowStateUtils.svelte'
	import type { RawScript, ScriptLang } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import ModuleAcceptReject, {
		aiModuleActionToBgColor,
		getAiModuleAction
	} from '$lib/components/copilot/chat/flow/ModuleAcceptReject.svelte'
	let {
		small,
		clazz
	}: {
		small: boolean
		clazz: string
	} = $props()

	const dispatch = createEventDispatcher<{
		generateStep: { moduleId: string; instructions: string; lang: ScriptLang }
	}>()

	const { selectedId, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

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
		refreshStateStore(flowStore)
	}

	const action = $derived(getAiModuleAction('failure'))
</script>

{#if flowStore.val?.value?.failure_module}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		id="flow-editor-error-handler"
		class={classNames(
			'z-10',
			'relative cursor-pointer border transition-colors duration-[400ms] ease-linear rounded-sm px-2 py-1 gap-2 bg-surface text-sm flex items-center flex-row',
			$selectedId?.includes('failure')
				? 'outline outline-offset-1 outline-2 outline-slate-900 dark:outline-slate-900/0 dark:bg-surface-secondary dark:border-gray-400'
				: '',
			aiModuleActionToBgColor(action)
		)}
		style="min-width: {flowStore.val?.value?.failure_module
			? small
				? '200px'
				: '230px'
			: ''}; max-width: 275px;"
		onclick={() => {
			if (flowStore.val?.value?.failure_module) {
				$selectedId = 'failure'
			}
		}}
		title={'Error Handler'}
	>
		<ModuleAcceptReject id="failure" {action} placement="bottom" />

		<div class="flex items-center grow-0 min-w-0 gap-2">
			<Bug size={16} color={'#3b82f6'} />
		</div>

		<div class="truncate grow min-w-0 text-center text-xs">
			{flowStore.val.value.failure_module?.summary ||
				(flowStore.val.value.failure_module?.value.type === 'rawscript'
					? `${flowStore.val.value.failure_module?.value.language}`
					: 'TBD')}
		</div>

		{#if !action}
			<button
				title="Delete failure script"
				type="button"
				class={twMerge(
					'w-5 h-4 flex items-center justify-center grow-0 shrink-0',
					'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
					'text-secondary',
					'bg-surface focus:outline-none hover:bg-surface-hover rounded '
				)}
				onclick={() => {
					flowStore.val.value.failure_module = undefined
					$selectedId = 'settings-metadata'
				}}
			>
				<X size={12} />
			</button>
		{/if}
	</div>
{:else}
	<InsertModuleButton
		index={0}
		placement={'bottom-center'}
		on:new={(e) => {
			insertFailureModule(e.detail.inlineScript)
		}}
		on:pickScript={(e) => {
			insertFailureModule(undefined, e.detail)
		}}
		kind="failure"
		clazz={twMerge(clazz, '!outline-none px-2 py-1.5')}
	/>
{/if}
