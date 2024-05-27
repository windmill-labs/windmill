<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'
	import InitializeComponent from './InitializeComponent.svelte'
	import type { CancelablePromise } from '$lib/gen'
	import type { SideEffectAction } from './types'
	import { handleSideEffect as x } from './handleSideEffect'

	export let componentInput: AppInput | undefined
	export let noInitialize = false
	export let hideRefreshButton: boolean | undefined = undefined
	export let overrideCallback: (() => CancelablePromise<void>) | undefined = undefined
	export let overrideAutoRefresh: boolean = false
	export let id: string
	export let result: any = undefined
	export let initializing: boolean = true
	export let loading: boolean = false
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
	export let forceSchemaDisplay: boolean = false
	export let runnableClass = ''
	export let runnableStyle = ''
	export let doOnSuccess: SideEffectAction = undefined
	export let doOnError: SideEffectAction = undefined

	export let render: boolean
	export let recomputeIds: string[] = []
	export let outputs: {
		result: Output<any>
		loading: Output<boolean>
		jobId?: Output<any> | undefined
	}
	export let extraKey: string | undefined = undefined
	export let refreshOnStart: boolean = false
	export let errorHandledByComponent: boolean = false
	export let hasChildrens: boolean = false
	export let allowConcurentRequests = false
	export let onSuccess: (result: any) => void = () => {}

	export function setArgs(value: any) {
		runnableComponent?.setArgs(value)
	}

	const { staticExporter, noBackend, runnableComponents, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}

	if (noBackend) {
		initializing = false
	}

	onMount(() => {
		$staticExporter[id] = () => {
			return result
		}
	})

	if (!(initializing && componentInput?.type === 'runnable' && isRunnableDefined(componentInput))) {
		initializing = false
	}

	// We need to make sure that old apps have correct values. Triggerable (button, form, etc) have both autoRefresh and recomputeOnInputChanged set to false
	$: if (!autoRefresh && componentInput?.type === 'runnable' && componentInput.autoRefresh) {
		componentInput.autoRefresh = false
		componentInput.recomputeOnInputChanged = false
	}

	function isRunnableDefined(componentInput) {
		return (
			(isScriptByNameDefined(componentInput) &&
				componentInput.runnable.inlineScript != undefined) ||
			isScriptByPathDefined(componentInput)
		)
	}

	export async function handleSideEffect(success: boolean, errorMessage?: string) {
		x(
			success ? doOnSuccess : doOnError,
			success,
			$runnableComponents,
			$componentControl,
			recomputeIds,
			errorMessage
		)
	}
</script>

{#if componentInput === undefined}
	{#if !noInitialize}
		<InitializeComponent {id} />
	{/if}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined(componentInput)}
	<RunnableComponent
		{noInitialize}
		{allowConcurentRequests}
		{refreshOnStart}
		{extraKey}
		{hasChildrens}
		bind:loading
		bind:this={runnableComponent}
		fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		hideRefreshButton={componentInput.hideRefreshButton ?? hideRefreshButton}
		transformer={componentInput.transformer}
		{autoRefresh}
		{overrideCallback}
		{overrideAutoRefresh}
		recomputableByRefreshButton={componentInput.autoRefresh ?? true}
		bind:recomputeOnInputChanged={componentInput.recomputeOnInputChanged}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
		{initializing}
		wrapperClass={runnableClass}
		wrapperStyle={runnableStyle}
		{render}
		on:started
		on:done
		on:doneError
		on:cancel
		on:argsChanged
		on:resultSet={() => (initializing = false)}
		on:success={(e) => {
			onSuccess(e.detail)
			x(doOnSuccess, true, $runnableComponents, $componentControl, recomputeIds)
		}}
		on:handleError={(e) => {
			x(doOnError, false, $runnableComponents, $componentControl, recomputeIds, e.detail)
		}}
		{outputs}
		{errorHandledByComponent}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent {noInitialize} {hasChildrens} {render} bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
