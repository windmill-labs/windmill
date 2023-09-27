<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faUser } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let customCss: ComponentCustomCSS<'formcomponent'> | undefined = undefined
	export let render: boolean
	export let errorHandledByComponent: boolean | undefined = false

	$: errorHandledByComponent = resolvedConfig?.onError?.selected !== 'errorOverlay'

	export const staticOutputs: string[] = ['loading', 'result', 'jobId']

	const { app, worldStore, stateId, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	const resolvedConfig = initConfig(
		components['formcomponent'].initialData.configuration,
		configuration
	)

	$componentControl[id] = {
		setValue(nvalue: string) {
			wrapper?.setArgs(nvalue)
		}
	}

	let runnableComponent: RunnableComponent
	let loading = false

	$: noInputs =
		$stateId != undefined &&
		(componentInput?.type != 'runnable' || Object.keys(componentInput?.fields ?? {}).length == 0)

	let css = initCss($app.css?.formcomponent, customCss)

	let wrapper: RunnableWrapper
</script>

{#each Object.keys(components['formcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.formcomponent}
	/>
{/each}

<RunnableWrapper
	{recomputeIds}
	{render}
	bind:this={wrapper}
	bind:runnableComponent
	bind:loading
	{componentInput}
	{id}
	doOnSuccess={resolvedConfig.onSuccess}
	doOnError={resolvedConfig.onError}
	{errorHandledByComponent}
	{extraQueryParams}
	autoRefresh={false}
	forceSchemaDisplay={true}
	runnableClass={css?.container?.class}
	runnableStyle={css?.container?.style}
	{outputs}
>
	<AlignWrapper {horizontalAlignment}>
		<div
			class={twMerge('flex flex-col gap-2 px-4 w-full', css?.container?.class, 'wm-submit')}
			style={css?.container?.style ?? ''}
		>
			<div>
				{#if noInputs}
					<div class="text-secondary italic text-sm my-4">
						Run forms are associated with a runnable that has user inputs.
						<br />
						Once a script or flow is chosen, set some <strong>Runnable Inputs</strong> to
						<strong>
							User Input
							<Icon data={faUser} scale={1.3} class="rounded-sm bg-surface-secondary p-1 ml-0.5" />
						</strong>
					</div>
				{/if}
			</div>
			<div class="flex justify-end my-1">
				{#if !noInputs}
					<Button
						{loading}
						btnClasses={twMerge(css?.button?.class, 'wm-submit-button')}
						style={css?.button?.style ?? ''}
						on:pointerdown={(e) => {
							e?.stopPropagation()
						}}
						on:click={() => {
							runnableComponent?.runComponent()
						}}
						size={resolvedConfig.size}
						color={resolvedConfig.color}
					>
						{resolvedConfig.label}
					</Button>
				{/if}
			</div>
		</div>
	</AlignWrapper>
</RunnableWrapper>
