<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import { GitFork, Pen, RefreshCcw, Save, Settings } from 'lucide-svelte'
	import Popover from '../../Popover.svelte'
	import DropdownV2 from '../../DropdownV2.svelte'
	import type { FlowEditorContext } from '../types'
	import { sendUserToast } from '$lib/utils'
	import { getLatestHashForScript } from '$lib/scripts'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowModuleWorkerTagSelect from './FlowModuleWorkerTagSelect.svelte'

	interface Props {
		module: FlowModule
		tag: string | undefined
	}

	let { module, tag }: Props = $props()
	const { scriptEditorDrawer, workspaceScriptSettingsDrawer, flowEditorDrawer, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
</script>

<div class="flex flex-row gap-2 whitespace-nowrap">
	{#if module.value.type === 'script'}
		{#if !module.value.path.startsWith('hub/') && customUi?.scriptEdit != false}
			<Popover notClickable placement="bottom">
				<Button
					unifiedSize="sm"
					variant="subtle"
					onClick={async () => {
						if (module.value.type == 'script') {
							const hash =
								module.value.hash ??
								(await getLatestHashForScript(module.value.path, opWorkspace?.()))
							$scriptEditorDrawer?.openDrawer(hash, () => {
								dispatch('reload')
								sendUserToast('Script has been updated')
							})
						}
					}}
					startIcon={{ icon: Pen }}
					iconOnly
					aria-label="Edit the script's code"
					disabled={module.value.hash != undefined}
				/>
				{#snippet text()}Edit the script's code{/snippet}
			</Popover>
			<!-- Only when the settings drawer is actually mounted (not in the local-dev
				editors, which provide the context store but never render it). -->
			{#if $workspaceScriptSettingsDrawer}
				<Popover notClickable placement="bottom">
					<Button
						unifiedSize="sm"
						variant="subtle"
						onClick={() => {
							if (module.value.type == 'script') {
								$workspaceScriptSettingsDrawer?.openDrawer(
									module.value.path,
									module.value.hash,
									() => {
										dispatch('reload')
									}
								)
							}
						}}
						startIcon={{ icon: Settings }}
						iconOnly
						aria-label="Edit the script's runtime settings"
						disabled={module.value.hash != undefined}
					/>
					{#snippet text()}Edit the script's runtime settings (concurrency, cache, timeout, ...){/snippet}
				</Popover>
			{/if}
		{/if}
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
		{#if customUi?.scriptFork != false}
			<Popover notClickable placement="bottom">
				<Button
					unifiedSize="sm"
					variant="subtle"
					on:click={() => dispatch('fork')}
					startIcon={{ icon: GitFork }}
					iconOnly
					aria-label="Fork into an inline script"
				/>
				{#snippet text()}Fork into an inline script{/snippet}
			</Popover>
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
			items={[
				{
					displayName: 'Save to workspace',
					icon: Save,
					action: () => dispatch('createScriptFromInlineScript')
				}
			]}
		/>
	{/if}
</div>
