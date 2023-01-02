<script lang="ts">
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { defaultIfEmptyString, emptySchema } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import { fieldTypeToTsType, schemaToInputsSpec } from '../../utils'
	import InputValue from './InputValue.svelte'
	import MissingConnectionWarning from './MissingConnectionWarning.svelte'
	import RefreshButton from './RefreshButton.svelte'

	// Component props
	export let id: string
	export let fields: AppInputs
	export let runnable: Runnable
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let result: any = undefined
	export let forceSchemaDisplay: boolean = false

	const { worldStore, runnableComponents, workspace, appPath } =
		getContext<AppEditorContext>('AppEditorContext')

	onMount(() => {
		if (autoRefresh) {
			$runnableComponents[id] = async () => {
				await executeComponent()
			}
		}
		executeComponent()
	})

	let args: Record<string, any> = {}
	let debouncedArgs: Record<string, any> = args
	let testIsLoading = false
	let runnableInputValues: Record<string, any> = {}

	let argsTimeout: NodeJS.Timeout | undefined

	function setDebouncedArgs() {
		argsTimeout && clearTimeout(argsTimeout)
		argsTimeout = setTimeout(() => {
			Object.assign(args, debouncedArgs)
			args = args
		}, 200)
	}
	$: debouncedArgs && setDebouncedArgs()

	let previousStaticArgs: any = {}
	function setStaticInputsToArgs() {
		let nargs = {}
		Object.entries(fields ?? {}).forEach(([key, value]) => {
			if (value.type === 'static') {
				nargs[key] = value.value
			}
		})

		if (JSON.stringify(previousStaticArgs) != JSON.stringify(nargs)) {
			previousStaticArgs = nargs
			Object.assign(args, nargs)
			args = args
		}
	}

	$: fields && setStaticInputsToArgs()

	function argMergedArgsValid(mergedArgs: Record<string, any>, testJobLoader) {
		if (!fields) {
			return false
		}

		if (
			Object.keys(fields).length !==
			Object.keys(mergedArgs).length - Object.keys(extraQueryParams).length
		) {
			return false
		}

		const areAllArgsValid = Object.values(mergedArgs).every(
			(arg) => arg !== undefined && arg !== null
		)

		if (areAllArgsValid && autoRefresh && testJobLoader) {
			executeComponent()
		}

		return areAllArgsValid
	}

	$: isValid =
		Object.keys(fields ?? {}).length == 0 ||
		argMergedArgsValid({ ...extraQueryParams, ...runnableInputValues, ...args }, testJobLoader)

	// Test job internal state
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<Array<any>>
		loading: Output<boolean>
	}

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	$: runnable?.type === 'runnableByName' && loadSchemaAndInputsByName()

	async function loadSchemaAndInputsByName() {
		if (runnable?.type === 'runnableByName') {
			const { inlineScript } = runnable
			// Inline scripts directly provide the schema
			if (inlineScript) {
				const newSchema = inlineScript.schema

				const newFields = reloadInputs(newSchema)

				if (JSON.stringify(newFields) !== JSON.stringify(fields)) {
					setTimeout(() => {
						fields = newFields
					}, 0)
				}
			}
		}
	}

	// When the schema is loaded, we need to update the inputs spec
	// in order to render the inputs the component panel
	function reloadInputs(schema: Schema) {
		let schemaWithoutExtraQueries: Schema = JSON.parse(JSON.stringify(schema))

		const result = {}
		const newInputs = schemaToInputsSpec(schemaWithoutExtraQueries)
		if (!fields) {
			return newInputs
		}
		Object.keys(newInputs).forEach((key) => {
			const newInput = newInputs[key]
			const oldInput = fields[key]

			// If the input is not present in the old inputs, add it
			if (oldInput === undefined) {
				result[key] = newInput
			} else {
				if (fieldTypeToTsType(newInput.fieldType) !== fieldTypeToTsType(oldInput.fieldType)) {
					result[key] = newInput
				} else {
					result[key] = oldInput
				}
			}
		})

		return result
	}

	$: schemaStripped = runnable && stripSchema(fields)

	function stripSchema(inputs: AppInputs): Schema {
		let schema =
			runnable?.type == 'runnableByName' ? runnable.inlineScript?.schema : runnable?.schema
		try {
			schemaStripped = JSON.parse(JSON.stringify(schema))
		} catch (e) {
			console.warn('Error loading schema')
			return emptySchema()
		}

		// Remove hidden static inputs
		Object.keys(inputs ?? {}).forEach((key: string) => {
			const input = inputs[key]

			if (input.type === 'static' && schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}

			if (input.type === 'connected' && schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}
		})
		return schemaStripped as Schema
	}

	$: disabledArgs = Object.keys(fields ?? {}).reduce(
		(disabledArgsAccumulator: string[], inputName: string) => {
			if (fields[inputName].type === 'static') {
				disabledArgsAccumulator = [...disabledArgsAccumulator, inputName]
			}
			return disabledArgsAccumulator
		},
		[]
	)

	async function executeComponent() {
		if (outputs?.loading.peak() === true) {
			return
		}

		if (runnable?.type === 'runnableByName' && !runnable.inlineScript) {
			return
		}

		outputs?.loading?.set(true)

		await testJobLoader?.abstractRun(() => {
			const requestBody = {
				args: { ...args, ...runnableInputValues },
				force_viewer_static_fields: {}
			}

			if (runnable?.type === 'runnableByName') {
				const { inlineScript } = runnable

				if (inlineScript) {
					requestBody['raw_code'] = {
						content: inlineScript.content,
						language: inlineScript.language,
						path: inlineScript.path
					}
				}
			} else if (runnable?.type === 'runnableByPath') {
				const { path, runType } = runnable
				requestBody['path'] = runType !== 'hubscript' ? `${runType}/${path}` : `script/${path}`
			}

			return AppService.executeComponent({
				workspace,
				path: defaultIfEmptyString(appPath, 'newapp'),
				requestBody
			})
		})
	}

	export async function runComponent() {
		await executeComponent()
	}
</script>

{#each Object.keys(fields ?? {}) as key}
	<InputValue
		{id}
		input={fields[key]}
		bind:value={runnableInputValues[key]}
		row={extraQueryParams['row'] ?? {}}
	/>
{/each}

<TestJobLoader
	workspaceOverride={workspace}
	on:done={() => {
		if (testJob && outputs) {
			outputs.result?.set(testJob?.result)
			outputs.loading?.set(false)
			result = testJob.result
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

<div class="h-full flex flex-col">
	{#if schemaStripped !== undefined && (autoRefresh || forceSchemaDisplay)}
		<div class="px-2">
			<SchemaForm
				schema={schemaStripped}
				bind:args={debouncedArgs}
				{isValid}
				{disabledArgs}
				shouldHideNoInputs
				noVariablePicker
			/>
		</div>
	{/if}

	{#if !runnable && autoRefresh}
		<Alert type="warning" size="xs" class="mt-2 px-1" title="Missing runnable">
			Please select a runnable
		</Alert>
	{:else if autoRefresh === true}
		<div class="flex absolute top-1 right-1">
			<RefreshButton componentId={id} />
		</div>

		{#if isValid}
			<slot />
		{:else}
			<Alert type="info" size="xs" class="mt-2 px-1" title="Missing inputs">
				Please fill in all the inputs

				{#each Object.keys(fields ?? {}) as key}
					{#if fields[key].type === 'connected'}
						<MissingConnectionWarning input={fields[key]} />
					{/if}
				{/each}
			</Alert>
		{/if}
	{:else}
		<slot />
	{/if}
</div>
