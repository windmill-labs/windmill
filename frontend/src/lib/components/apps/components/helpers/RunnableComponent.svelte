<script lang="ts">
	import { page } from '$app/stores'
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import { fieldTypeToTsType, loadSchema, schemaToInputsSpec } from '../../utils'
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

	const { worldStore, runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	onMount(() => {
		if (autoRefresh) {
			$runnableComponents[id] = async () => {
				await executeComponent()
			}
		}
	})

	let pagePath = $page.params.path
	let args: Record<string, any> = {}
	let schema: Schema | undefined = undefined
	let testIsLoading = false
	let runnableInputValues: Record<string, any> = {}

	function setStaticInputsToArgs() {
		let nargs = {}
		Object.entries(fields ?? {}).forEach(([key, value]) => {
			if (value.type === 'static') {
				nargs[key] = value.value
			}
		})

		if (JSON.stringify(args) != JSON.stringify(nargs)) {
			args = nargs
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

	async function loadSchemaFromTriggerable(
		workspace: string,
		path: string,
		runType: 'script' | 'flow' | 'hubscript'
	): Promise<Schema> {
		return loadSchema(workspace, path, runType) ?? emptySchema()
	}

	$: runnable?.type === 'runnableByName' && loadSchemaAndInputsByName()
	$: !schema && runnable?.type === 'runnableByPath' && loadSchemaAndInputsByPath()

	async function loadSchemaAndInputsByName() {
		if (runnable?.type === 'runnableByName') {
			const { inlineScript } = runnable
			// Inline scripts directly provide the schema
			if (inlineScript) {
				const newSchema = inlineScript.schema
				schema = newSchema

				const newFields = reloadInputs(newSchema)

				if (JSON.stringify(newFields) !== JSON.stringify(fields)) {
					fields = newFields
					setTimeout(() => {
						fields = newFields
					}, 0)
				}
			}
		}
	}

	async function loadSchemaAndInputsByPath() {
		if ($workspaceStore && runnable?.type === 'runnableByPath') {
			// Remote schema needs to be loaded
			const { path, runType } = runnable
			const newSchema = await loadSchemaFromTriggerable($workspaceStore, path, runType)
			schema = newSchema

			let schemaWithoutExtraQueries: Schema = JSON.parse(JSON.stringify(schema))

			// Remove extra query params from the schema, which are not directly configurable by the user
			Object.keys(extraQueryParams).forEach((key) => {
				delete schemaWithoutExtraQueries.properties[key]
			})

			fields = schemaToInputsSpec(schemaWithoutExtraQueries)
		}
	}

	// When the schema is loaded, we need to update the inputs spec
	// in order to render the inputs the component panel
	function reloadInputs(schema: Schema) {
		let schemaWithoutExtraQueries: Schema = JSON.parse(JSON.stringify(schema))

		// Remove extra query params from the schema, which are not directly configurable by the user
		Object.keys(extraQueryParams).forEach((key) => {
			delete schemaWithoutExtraQueries.properties[key]
		})

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

	let schemaStripped: Schema | undefined = undefined

	function stripSchema(schema: Schema, inputs: AppInputs) {
		schemaStripped = JSON.parse(JSON.stringify(schema))

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

		// Remove extra query params from schema
		Object.keys(extraQueryParams).forEach((key: string) => {
			if (schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}
		})
	}

	$: schema && fields && stripSchema(schema, fields)

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
		if (!schema) {
			return
		}

		if (outputs?.loading.peak() === true) {
			return
		}

		outputs?.loading?.set(true)

		await testJobLoader?.abstractRun(() => {
			const requestBody = {
				args: { ...extraQueryParams, ...args, ...runnableInputValues },
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
				workspace: $workspaceStore!,
				path: pagePath,
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
		<SchemaForm
			schema={schemaStripped}
			bind:args
			{isValid}
			{disabledArgs}
			shouldHideNoInputs
			noVariablePicker
		/>
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
