<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, onDestroy, untrack } from 'svelte'
	import { initConfig, initOutput, selectId } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { Schema } from '$lib/common'
	import { initCss } from '../../utils'
	import { twMerge } from 'tailwind-merge'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { deepEqual } from 'fast-equals'
	import { computeWorkspaceS3FileInputPolicy } from '../../editor/appUtilsS3'
	import { defaultIfEmptyString } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		initializing?: boolean | undefined
		render: boolean
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'schemaformcomponent'> | undefined
	}

	let {
		id,
		componentInput,
		initializing = $bindable(undefined),
		render,
		configuration,
		customCss = undefined
	}: Props = $props()

	const {
		worldStore,
		connectingInput,
		app,
		selectedComponent,
		componentControl,
		appPath,
		isEditor,
		workspace
	} = getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		valid: true,
		values: {}
	})

	let result: Schema | undefined = $state(undefined)
	let args: Record<string, unknown> = $state(!iterContext ? (outputs?.values?.peak() ?? {}) : {})

	function handleArgsChange() {
		const newArgs: Record<string, unknown> = {}

		for (const key in args) {
			if (result?.properties[key]) {
				newArgs[key] = args[key]
			}
		}

		outputs.values.set(newArgs, true)
		if (iterContext && listInputs) {
			listInputs.set(id, newArgs)
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let schemaForm: SchemaForm | undefined = $state()

	$componentControl[id] = {
		setValue(nvalue: any) {
			args = nvalue
			outputs.values.set(nvalue, false)
		},
		invalidate(key: string, error: string) {
			schemaForm?.invalidate(key, error)
		},
		validateAll() {
			schemaForm?.validateAll()
		},
		validate(key: string) {
			schemaForm?.validate(key)
		}
	}

	let css = $state(initCss($app.css?.schemaformcomponent, customCss))

	const resolvedConfig = $state(
		initConfig(components['schemaformcomponent'].initialData.configuration, configuration)
	)

	let valid = $state(true)

	let previousDefault = $state(resolvedConfig.defaultValues)

	function onDefaultChange() {
		previousDefault = structuredClone(resolvedConfig.defaultValues)
		args = previousDefault ?? {}
	}

	function computeS3ForceViewerPolicies() {
		if (!isEditor) {
			return undefined
		}
		const policy = computeWorkspaceS3FileInputPolicy()
		return policy
	}
	$effect(() => {
		args && untrack(() => handleArgsChange())
	})
	$effect(() => {
		outputs.valid.set(valid)
	})
	$effect(() => {
		resolvedConfig.defaultValues &&
			!deepEqual(previousDefault, resolvedConfig.defaultValues) &&
			onDefaultChange()
	})
</script>

{#each Object.keys(components['schemaformcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.schemaformcomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if result && Object.keys(result?.properties ?? {}).length > 0}
		<div
			class={twMerge('p-2 overflow-auto h-full', css?.container?.class, 'wm-schema-form')}
			style={css?.container?.style}
		>
			<div
				onpointerdown={stopPropagation(
					(e) =>
						!$connectingInput.opened && selectId(e as PointerEvent, id, selectedComponent, $app)
				)}
			>
				<SchemaForm
					noVariablePicker
					onlyMaskPassword
					defaultValues={resolvedConfig.defaultValues}
					dynamicEnums={resolvedConfig.dynamicEnums}
					schema={result}
					bind:isValid={valid}
					bind:args
					bind:this={schemaForm}
					displayType={Boolean(resolvedConfig.displayType)}
					largeGap={Boolean(resolvedConfig.largeGap)}
					appPath={defaultIfEmptyString($appPath, `u/${$userStore?.username ?? 'unknown'}/newapp`)}
					{computeS3ForceViewerPolicies}
					{workspace}
					{css}
				/>
			</div>
		</div>
	{:else}
		<p class="m-2 italic">Empty form (no property)</p>
	{/if}
</RunnableWrapper>
