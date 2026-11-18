<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import { Pen, RefreshCcw, Save } from 'lucide-svelte'
	import type { FlowEditorContext } from '../types'
	import { sendUserToast } from '$lib/utils'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowModuleWorkerTagSelect from './FlowModuleWorkerTagSelect.svelte'

	interface Props {
		module: FlowModule
		tag: string | undefined
	}

	let { module, tag }: Props = $props()
	const { flowEditorDrawer } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
</script>

<div class="flex shrink-0 flex-row items-center gap-2 whitespace-nowrap">
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

	{#if (module.value.type === 'aiagent' || module.value.type === 'aidecision') && customUi?.tagEdit != false}
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
		{#if customUi?.saveToWorkspace != false && customUi?.editorBar?.saveToWorkspace != false}
			<Button
				unifiedSize="sm"
				variant="subtle"
				startIcon={{ icon: Save }}
				on:click={() => dispatch('createScriptFromInlineScript')}
				iconOnly={true}
				title="Save to workspace"
			/>
		{/if}
	{/if}
</div>
