<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import { Pen, RefreshCcw, Save } from 'lucide-svelte'
	import DropdownV2 from '../../DropdownV2.svelte'
	import type { FlowEditorContext } from '../types'
	import { sendUserToast } from '$lib/utils'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowModuleWorkerTagSelect from './FlowModuleWorkerTagSelect.svelte'
	import { useFlowEditorTelemetry } from '../flowEditorTelemetry'

	interface Props {
		module: FlowModule
		tag: string | undefined
	}

	let { module, tag }: Props = $props()
	const { flowEditorDrawer } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

	// The menu is counted alongside the action it holds: "Save to workspace" no longer has a
	// button of its own, so how often it is reached at all is what says whether it still is.
	const telemetry = useFlowEditorTelemetry()
	let menuOpen = $state(false)
	$effect(() => {
		if (menuOpen) telemetry.log('header_action', 'menu_open')
	})
</script>

<div class="flex shrink-0 flex-row gap-2 whitespace-nowrap">
	{#if module.value.type === 'script'}
		{#if customUi?.tagEdit != false}
			<FlowModuleWorkerTagSelect
				isPreprocessor={module.id == 'preprocessor'}
				placeholder={customUi?.tagSelectPlaceholder}
				noLabel={customUi?.tagSelectNoLabel}
				nullTag={tag}
				tag={module.value.tag_override}
				on:change={(e) => dispatch('tagChange', e.detail)}
			/>
		{/if}
	{:else if module.value.type === 'flow'}
		<Button
			unifiedSize="sm"
			variant="subtle"
			on:click={async () => {
				if (module.value.type == 'flow') {
					$flowEditorDrawer?.openDrawer(module.value.path, () => {
						dispatch('reload')
						sendUserToast('Flow has been updated')
					})
				}
			}}
			startIcon={{ icon: Pen }}
			iconOnly={false}
		>
			Edit
		</Button>
		<Button
			unifiedSize="sm"
			variant="subtle"
			on:click={async () => {
				dispatch('reload')
			}}
			startIcon={{
				icon: RefreshCcw
			}}
			iconOnly={true}
		/>
	{/if}

	{#if module.value.type === 'aiagent' && customUi?.tagEdit != false}
		<FlowModuleWorkerTagSelect
			isPreprocessor={false}
			placeholder={customUi?.tagSelectPlaceholder}
			noLabel={customUi?.tagSelectNoLabel}
			nullTag={tag}
			tag={module.value.tag}
			on:change={(e) => dispatch('tagChange', e.detail)}
		/>
	{/if}

	{#if module.value.type === 'rawscript'}
		<FlowModuleWorkerTagSelect
			isPreprocessor={module.id == 'preprocessor'}
			placeholder={customUi?.tagSelectPlaceholder}
			noLabel={customUi?.tagSelectNoLabel}
			nullTag={tag}
			tag={module.value.tag}
			on:change={(e) => dispatch('tagChange', e.detail)}
		/>
		<DropdownV2
			size="sm"
			placement="bottom-end"
			bind:open={menuOpen}
			items={[
				{
					displayName: 'Save to workspace',
					icon: Save,
					action: () => {
						telemetry.log('header_action', 'save_to_workspace')
						dispatch('createScriptFromInlineScript')
					}
				}
			]}
		/>
	{/if}
</div>
