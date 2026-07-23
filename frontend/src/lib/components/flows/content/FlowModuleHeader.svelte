<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import {
		Bed,
		Database,
		Gauge,
		GitFork,
		Pen,
		PhoneIncoming,
		RefreshCcw,
		Repeat,
		Square,
		Pin,
		Save,
		Settings
	} from 'lucide-svelte'
	import Popover from '../../Popover.svelte'
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

	let popoverClasses =
		'center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
</script>

<div class="flex flex-row gap-2 whitespace-nowrap">
	{#if module.value.type === 'script' || module.value.type === 'rawscript' || module.value.type == 'flow'}
		{#if module.retry?.constant || module.retry?.exponential}
			<Popover placement="bottom" class={popoverClasses} onClick={() => dispatch('toggleRetry')}>
				<Repeat size={14} />
				{#snippet text()}
					Retries
				{/snippet}
			</Popover>
		{/if}
		{#if module?.value?.['concurrent_limit'] != undefined}
			<Popover
				placement="bottom"
				class={popoverClasses}
				onClick={() => dispatch('toggleConcurrency')}
			>
				<Gauge size={14} />
				{#snippet text()}
					Concurrency Limits
				{/snippet}
			</Popover>
		{/if}
		{#if module.cache_ttl != undefined}
			<Popover placement="bottom" class={popoverClasses} onClick={() => dispatch('toggleCache')}>
				<Database size={14} />
				{#snippet text()}
					Cache
				{/snippet}
			</Popover>
		{/if}
		{#if module.stop_after_if || module.stop_after_all_iters_if}
			<Popover
				placement="bottom"
				class={popoverClasses}
				onClick={() => dispatch('toggleStopAfterIf')}
			>
				<Square size={14} />
				{#snippet text()}
					Early stop/break
				{/snippet}
			</Popover>
		{/if}
		{#if module.suspend}
			<Popover placement="bottom" class={popoverClasses} onClick={() => dispatch('toggleSuspend')}>
				<PhoneIncoming size={14} />
				{#snippet text()}
					Suspend
				{/snippet}
			</Popover>
		{/if}
		{#if module.sleep}
			<Popover placement="bottom" class={popoverClasses} onClick={() => dispatch('toggleSleep')}>
				<Bed size={14} />
				{#snippet text()}
					Sleep
				{/snippet}
			</Popover>
		{/if}
		{#if module.mock?.enabled}
			<Popover placement="bottom" class={popoverClasses} onClick={() => dispatch('togglePin')}>
				<Pin size={14} />
				{#snippet text()}
					This step is pinned
				{/snippet}
			</Popover>
		{/if}
	{/if}
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
		<Button
			unifiedSize="sm"
			variant="subtle"
			startIcon={{ icon: Save }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			iconOnly={false}
		>
			Save to workspace
		</Button>
	{/if}
</div>
