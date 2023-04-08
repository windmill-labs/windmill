<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/utils'

	export let componentInput: AppInput | undefined
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
	export let doOnSuccess:
		| {
				selected: 'gotoUrl' | 'none' | 'setTab' | 'sendToast'
				configuration: {
					gotoUrl: { url: string | undefined; newTab: boolean | undefined }
					setTab: {
						setTab: { id: string; index: number }[] | undefined
					}
					sendToast: {
						message: string | undefined
					}
				}
		  }
		| undefined = undefined

	export let render: boolean
	export let recomputeIds: string[] = []
	export let outputs: { result: Output<any>; loading: Output<boolean> }
	export let extraKey: string | undefined = undefined

	const { staticExporter, noBackend, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	$: if (initializing && result != undefined) {
		initializing = false
	}

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}
	onMount(() => {
		$staticExporter[id] = () => result
	})

	function isRunnableDefined(componentInput) {
		return isScriptByNameDefined(componentInput) || isScriptByPathDefined(componentInput)
	}

	export function onSuccess() {
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb())
		}
		if (!doOnSuccess) return

		if (doOnSuccess.selected == 'none') return

		if (doOnSuccess.selected == 'setTab') {
			if (Array.isArray(doOnSuccess.configuration.setTab.setTab)) {
				doOnSuccess.configuration.setTab?.setTab?.forEach((tab) => {
					if (tab) {
						const { id, index } = tab
						$componentControl[id].setTab?.(index)
					}
				})
			}
		} else if (
			doOnSuccess.selected == 'gotoUrl' &&
			doOnSuccess.configuration.gotoUrl.url &&
			doOnSuccess.configuration.gotoUrl.url != ''
		) {
			if (doOnSuccess.configuration.gotoUrl.newTab) {
				window.open(doOnSuccess.configuration.gotoUrl.url, '_blank')
			} else {
				goto(doOnSuccess.configuration.gotoUrl.url)
			}
		} else if (
			doOnSuccess.selected == 'sendToast' &&
			doOnSuccess.configuration.sendToast.message &&
			doOnSuccess.configuration.sendToast.message != ''
		) {
			sendUserToast(doOnSuccess.configuration.sendToast.message)
		}
	}
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined(componentInput)}
	<RunnableComponent
		{extraKey}
		bind:loading
		bind:this={runnableComponent}
		fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		transformer={componentInput.transformer}
		{autoRefresh}
		bind:recomputeOnInputChanged={componentInput.recomputeOnInputChanged}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
		{initializing}
		wrapperClass={runnableClass}
		wrapperStyle={runnableStyle}
		{render}
		on:success={onSuccess}
		{outputs}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent {render} bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
