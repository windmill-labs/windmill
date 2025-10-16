<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import { Bug, X } from 'lucide-svelte'
	import InsertModulePopover from '$lib/components/flows/map/InsertModulePopover.svelte'
	import { insertNewFailureModule } from '$lib/components/flows/flowStateUtils.svelte'
	import type { RawScript, ScriptLang } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import ModuleAcceptReject, {
		getAiModuleAction
	} from '$lib/components/copilot/chat/flow/ModuleAcceptReject.svelte'
	import {
		aiModuleActionToBgColor,
		aiModuleActionToBorderColor,
		aiModuleActionToTextColor
	} from '$lib/components/copilot/chat/flow/utils'
	import Button from '$lib/components/common/button/Button.svelte'

	let {
		disableAi,
		small
	}: {
		small: boolean
		disableAi?: boolean
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
	<Button
		variant="default"
		wrapperClasses={twMerge('min-w-36 h-7', small ? 'max-w-52' : 'max-w-64')}
		btnClasses={twMerge(
			aiModuleActionToBgColor(action),
			aiModuleActionToBorderColor(action),
			aiModuleActionToTextColor(action)
		)}
		id="flow-editor-error-handler"
		selected={$selectedId?.includes('failure')}
		onClick={() => {
			if (flowStore.val?.value?.failure_module) {
				$selectedId = 'failure'
			}
		}}
	>
		<ModuleAcceptReject id="failure" {action} placement="bottom" />

		<Bug size={14} class="shrink-0" />

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
				class="ml-1"
				onclick={() => {
					flowStore.val.value.failure_module = undefined
					$selectedId = 'settings-metadata'
				}}
			>
				<X size={12} />
			</button>
		{/if}
	</Button>
{:else}
	<!-- Index 0 is used by the tutorial to identify the first "Add step" -->
	<InsertModulePopover
		{disableAi}
		placement={'bottom-center'}
		on:new={(e) => {
			insertFailureModule(e.detail.inlineScript)
		}}
		on:pickScript={(e) => {
			insertFailureModule(undefined, e.detail)
		}}
		kind="failure"
	>
		{#snippet trigger({ toggleOpen })}
			<Button
				size="xs"
				wrapperClasses="h-full min-w-36 h-7"
				title={`Add failure module`}
				variant="default"
				id={`flow-editor-add-step-error-handler-button`}
				onClick={() => toggleOpen()}
			>
				<div class="flex items-center gap-1">
					<Bug size={14} />
					<span class="text-xs w-20">Error Handler</span>
				</div>
			</Button>
		{/snippet}
	</InsertModulePopover>
{/if}
