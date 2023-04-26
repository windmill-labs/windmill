<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { ResultAppInput } from '../../inputType'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { clearResultAppInput } from '../../utils'
	import type { AppComponent } from '../component'
	import ComponentTriggerList from './triggerLists/ComponentTriggerList.svelte'
	import { ExternalLink, FunctionSquare } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/Popover.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	export let appInput: ResultAppInput
	export let appComponent: AppComponent

	$: if (appInput.autoRefresh === undefined) {
		appInput.autoRefresh = true
	}

	function detach() {
		if (appInput.runnable?.type === 'runnableByName' && appInput.runnable.inlineScript) {
			$app.unusedInlineScripts.push({
				name: appInput.runnable.name,
				inlineScript: appInput.runnable.inlineScript
			})
			$app = $app
			appInput = clearResultAppInput(appInput)
		}
	}

	function clear() {
		appInput = clearResultAppInput(appInput)
	}

	$: {
		if (appInput.recomputeOnInputChanged === undefined) {
			appInput.recomputeOnInputChanged = true
		}
		//TODO: remove after migration is done
		if (appInput.doNotRecomputeOnInputChanged != undefined) {
			appInput.recomputeOnInputChanged = !appInput.doNotRecomputeOnInputChanged
			appInput.doNotRecomputeOnInputChanged = undefined
		}
	}
</script>

<div class="flex gap-1 justify-end">
	{#if appInput.runnable?.type === 'runnableByName' && appInput.runnable.inlineScript}
		<Popover>
			<Button size="xs" color="light" variant="border" on:click={detach}>
				<div class="flex flex-row gap-1 items-center">
					<ExternalLink size={16} />
					Detach
				</div>
			</Button>
			<svelte:fragment slot="text">
				Detaching an inline script keep it for later to be reused by another component
			</svelte:fragment>
		</Popover>
	{/if}
	<Button size="xs" color="red" variant="border" startIcon={{ icon: faClose }} on:click={clear}>
		Clear
	</Button>
</div>

<div class="flex w-full flex-col border divide-y">
	<div class="flex flex-row p-2 border-gray-200 justify-between bg-blue-50/60">
		<div class="flex flex-row gap-2 items-center">
			<FunctionSquare size={16} color="blue" />
			<span class="text-xs font-semibold truncate">
				{#if appInput.runnable?.type === 'runnableByName'}
					{appInput.runnable.name}
				{:else if appInput.runnable?.type === 'runnableByPath'}
					{appInput.runnable.path}
				{/if}
			</span>
		</div>

		<Badge color="blue">
			{appInput.runnable?.type === 'runnableByName' &&
			appInput.runnable.inlineScript?.language === 'frontend'
				? 'Frontend'
				: 'Backend'}
		</Badge>
	</div>
	<div class="p-2">
		<div class="mb-2 text-sm font-semibold justify-between flex flex-row items-center">
			Transformer

			<Tooltip>
				A transformer is an optional frontend script that is executed right after the component's
				script whose purpose is to do lightweight transformation in the browser. It takes the
				previous computation's result as `result`
			</Tooltip>
		</div>

		<div class="flex gap-1 justify-between items-center">
			<span class="text-xs truncate"> Add a transformer </span>
			<div class="flex gap-1">
				<input
					value={appInput.transformer}
					type="checkbox"
					class="!w-4 !h-4 !rounded-sm"
					on:click={() => {
						if (appInput.transformer) {
							appInput.transformer = undefined
						} else {
							appInput.transformer = {
								language: 'frontend',
								content: 'return result'
							}
							$selectedComponentInEditor = appComponent.id + '_transformer'
						}
					}}
				/>
			</div>
		</div>
	</div>

	{#if !['buttoncomponent', 'formbuttoncomponent', 'formcomponent'].includes(appComponent.type)}
		<div class="p-2 flex flex-col">
			<div class="mb-2 text-sm font-semibold">Run configuration</div>
			<div class="flex flex-col gap-2">
				{#if appInput.autoRefresh}
					<div class="flex items-center justify-between w-full">
						<div class="flex flex-row items-center gap-2 text-xs">
							Run on start and app refresh
							<Tooltip>
								You may want to disable this so that the background script is only triggered by
								changes to other values or triggered by another computation on a button (See
								'Recompute Others')
							</Tooltip>
						</div>
						<input
							type="checkbox"
							bind:checked={appInput.autoRefresh}
							class="!w-4 !h-4 !rounded-sm"
						/>
					</div>
				{/if}
				<div class="flex items-center justify-between w-full">
					<div class="flex flex-row items-center gap-2 text-xs">
						Recompute on any input changes
					</div>
					<input
						type="checkbox"
						bind:checked={appInput.recomputeOnInputChanged}
						class="!w-4 !h-4 !rounded-sm"
					/>
				</div>
			</div>
		</div>
	{/if}
	{#if appInput.runnable?.type === 'runnableByName'}
		<div class="p-1">
			<ComponentTriggerList
				bind:runnable={appInput.runnable}
				{appComponent}
				fields={appInput.fields}
				recomputeOnInputChanged={appInput.recomputeOnInputChanged}
				autoRefresh={appInput.autoRefresh}
			/>
		</div>
	{/if}

	{#if appInput.runnable?.type === 'runnableByName' && !appInput.runnable.inlineScript}
		<span class="text-xs text-gray-500">
			Please configure the language in the inline script panel
		</span>
	{/if}
</div>
