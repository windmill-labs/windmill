<script lang="ts">
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { defaultIfEmptyString, emptySchema, sendUserToast } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import { fieldTypeToTsType, schemaToInputsSpec } from '../../utils'
	import InputValue from './InputValue.svelte'
	import RefreshButton from './RefreshButton.svelte'

	// Component props
	export let id: string
	export let fields: AppInputs
	export let runnable: Runnable
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let result: any = undefined
	export let forceSchemaDisplay: boolean = false
	export let noMinH = false
	export let defaultUserInput = false
	export let flexWrap = false
	export let wrapperClass = ''

	const { worldStore, runnableComponents, workspace, appPath, isEditor, jobs, noBackend } =
		getContext<AppEditorContext>('AppEditorContext')

	onMount(() => {
		if (autoRefresh) {
			$runnableComponents[id] = async () => {
				await executeComponent(true)
			}
			executeComponent(true)
		}
	})

	let args: Record<string, any> = {}
	let testIsLoading = false
	let runnableInputValues: Record<string, any> = {}

	let executeTimeout: NodeJS.Timeout | undefined = undefined

	function setDebouncedExecute() {
		executeTimeout && clearTimeout(executeTimeout)
		executeTimeout = setTimeout(() => {
			executeComponent(true)
		}, 200)
	}

	function computeStaticValues() {
		return Object.entries(fields ?? {})
			.filter(([k, v]) => v.type == 'static')
			.map(([name, field]) => {
				return [name, field['value']]
			})
	}

	let lazyStaticValues = computeStaticValues()
	let currentStaticValues = lazyStaticValues

	$: fields && (currentStaticValues = computeStaticValues())
	$: if (JSON.stringify(currentStaticValues) != JSON.stringify(lazyStaticValues)) {
		lazyStaticValues = currentStaticValues

		if (autoRefresh) {
			setDebouncedExecute()
		}
	}

	$: fields && (lazyStaticValues = computeStaticValues())
	$: runnableInputValues &&
		extraQueryParams &&
		args &&
		autoRefresh &&
		testJobLoader &&
		setDebouncedExecute()

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

	$: outputs?.loading?.set(testIsLoading)

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
		let schemaCopy: Schema = JSON.parse(JSON.stringify(schema))

		const result = {}
		const newInputs = schemaToInputsSpec(schemaCopy, defaultUserInput)
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
				if (
					fieldTypeToTsType(newInput.fieldType) !== fieldTypeToTsType(oldInput.fieldType) ||
					newInput.format !== oldInput.format ||
					newInput.subFieldType !== oldInput.subFieldType
				) {
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

	async function executeComponent(noToast = false) {
		if (noBackend) {
			if (!noToast) {
				sendUserToast('This app is not connected to a windmill backend, it is a static preview')
			}
			return
		}
		if (runnable?.type === 'runnableByName' && !runnable.inlineScript) {
			return
		}

		outputs?.loading?.set(true)

		let njob = await testJobLoader?.abstractRun(() => {
			const nonStaticRunnableInputs = {}
			const staticRunnableInputs = {}
			Object.keys(fields ?? {}).forEach((k) => {
				let field = fields[k]
				if (field?.type == 'static' && fields[k]) {
					staticRunnableInputs[k] = field.value
				} else if (field?.type == 'user') {
					nonStaticRunnableInputs[k] = args[k]
				} else {
					nonStaticRunnableInputs[k] = runnableInputValues[k]
				}
			})

			const requestBody = {
				args: nonStaticRunnableInputs,
				force_viewer_static_fields: !isEditor ? undefined : staticRunnableInputs
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
		if (njob) {
			$jobs = [{ job: njob, component: id }, ...$jobs]
		}
	}

	export async function runComponent() {
		await executeComponent()
	}
	let lastStartedAt: number = Date.now()
</script>

{#each Object.entries(fields ?? {}) as [key, v]}
	{#if v.type != 'static' && v.type != 'user'}
		<InputValue
			{id}
			input={fields[key]}
			bind:value={runnableInputValues[key]}
			row={extraQueryParams['row'] ?? {}}
		/>
	{/if}
{/each}

<TestJobLoader
	workspaceOverride={workspace}
	on:done={(e) => {
		if (testJob && outputs) {
			const startedAt = new Date(testJob.started_at).getTime()
			if (startedAt > lastStartedAt) {
				lastStartedAt = startedAt
				outputs.result?.set(testJob?.result)
				result = testJob.result
			}
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

<div class="h-full flex relative flex-row flex-wrap {wrapperClass}">
	{#if autoRefresh === true}
		<div class="flex absolute top-1 right-1">
			<RefreshButton componentId={id} />
		</div>
	{/if}
	{#if schemaStripped && Object.keys(schemaStripped?.properties ?? {}).length > 0 && (autoRefresh || forceSchemaDisplay)}
		<div class="px-2 h-fit min-h-0">
			<SchemaForm
				{flexWrap}
				schema={schemaStripped}
				bind:args
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
	{:else if result?.error}
		<div class="p-2">
			<Alert type="error" title="Error during execution">
				See "Debug Runs" on the top right for more details
				<pre
					title={JSON.stringify(result.error, null, 4)}
					class=" mt-2 text-2xs whitespace-pre-wrap">{JSON.stringify(result.error, null, 4)}</pre
				>
			</Alert>
			<slot />
		</div>
	{:else}
		<div class="block w-full h-full">
			<slot />
		</div>
	{/if}
</div>
