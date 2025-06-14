<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { User } from 'lucide-svelte'
	import { initCss } from '../../utils'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import AlwaysMountedModal from '$lib/components/common/modal/AlwaysMountedModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		recomputeIds?: string[] | undefined
		extraQueryParams?: Record<string, any>
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'formbuttoncomponent'> | undefined
		render: boolean
		errorHandledByComponent?: boolean | undefined
	}

	let {
		id,
		componentInput,
		configuration,
		recomputeIds = undefined,
		extraQueryParams = {},
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		customCss = undefined,
		render,
		errorHandledByComponent = $bindable(false)
	}: Props = $props()

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		onDelete: () => {
			modal?.close()
		},
		invalidate(key: string, error: string) {
			runnableComponent?.invalidate(key, error)
		},
		validateAll() {
			runnableComponent?.validateAll()
		},
		validate(key: string) {
			runnableComponent?.validate(key)
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let resolvedConfig = $state(
		initConfig(components['formbuttoncomponent'].initialData.configuration, configuration)
	)
	let runnableComponent: RunnableComponent | undefined = $state()

	let errors: Record<string, string> = {}

	let css = $state(initCss($app?.css?.formbuttoncomponent, customCss))
	let runnableWrapper: RunnableWrapper | undefined = $state()
	let loading = $state(false)
	let modal: AlwaysMountedModal | undefined = $state()
	$effect(() => {
		errorHandledByComponent = resolvedConfig?.onError?.selected !== 'errorOverlay'
	})
	let errorsMessage = $derived(
		Object.values(errors)
			.filter((x) => x != '')
			.join('\n')
	)
	let noInputs = $derived(
		componentInput?.type != 'runnable' || Object.keys(componentInput?.fields ?? {}).length == 0
	)
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.formbuttoncomponent}
	/>
{/each}

{#each Object.keys(components['formbuttoncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}
{#if render}
	<AlwaysMountedModal {css} title={resolvedConfig.modalTitle ?? ''} bind:this={modal}>
		<div class="flex flex-col gap-2 px-4 w-full pt-2">
			<RunnableWrapper
				bind:this={runnableWrapper}
				bind:loading
				{recomputeIds}
				{render}
				bind:runnableComponent
				{componentInput}
				{id}
				{extraQueryParams}
				autoRefresh={false}
				forceSchemaDisplay={true}
				runnableClass="!block"
				{outputs}
				doOnSuccess={resolvedConfig.onSuccess}
				doOnError={resolvedConfig.onError}
				{errorHandledByComponent}
			>
				{#if noInputs}
					<div class="text-secondary italic text-sm my-4">
						Run forms are associated with a runnable that has user inputs.
						<br />
						Once a script or flow is chosen, set some <strong>Runnable Inputs</strong> to
						<strong>
							User Input
							<User size={20} class="rounded-sm bg-gray-200 p-1 ml-0.5" />
						</strong>
					</div>
				{/if}
				<div class="flex justify-end gap-3 p-2">
					<Button
						{loading}
						btnClasses="my-1"
						on:pointerdown={(e) => {
							e?.stopPropagation()
						}}
						on:click={async () => {
							if (!runnableComponent) {
								runnableWrapper?.handleSideEffect(true)
							} else {
								await runnableComponent?.runComponent()
							}
							modal?.close()
						}}
						size="xs"
						color="dark"
					>
						Submit
					</Button>
				</div>
			</RunnableWrapper>
		</div>
	</AlwaysMountedModal>

	<AlignWrapper {render} {horizontalAlignment} {verticalAlignment}>
		{#if errorsMessage}
			<div class="text-red-500 text-xs">{errorsMessage}</div>
		{/if}
		<Button
			disabled={resolvedConfig.disabled ?? false}
			size={resolvedConfig.size ?? 'md'}
			color={resolvedConfig.color}
			btnClasses={twMerge(css?.button?.class, 'wm-button', 'wm-modal-form-button')}
			style={css?.button?.style ?? ''}
			on:click={(e) => {
				modal?.open()
			}}
		>
			{resolvedConfig.label}
		</Button>
	</AlignWrapper>
{:else}
	<RunnableWrapper {outputs} {render} {componentInput} {id} />
{/if}
