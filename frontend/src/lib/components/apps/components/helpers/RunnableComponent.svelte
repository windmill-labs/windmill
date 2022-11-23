<script lang="ts">
	import { page } from '$app/stores'
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faArrowsRotate } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { Output } from '../../rx'
	import type { AppEditorContext, InputsSpec } from '../../types'
	import { loadSchema, schemaToInputsSpec } from '../../utils'
	import InputValue from './InputValue.svelte'

	// Component props
	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let result: any = undefined

	const { app, worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let pagePath = $page.params.path
	let args: Record<string, any> = {}
	let schema: Schema | undefined = undefined
	let testIsLoading = false
	let runnableInputValues: Record<string, any> = {}

	$: mergedArgs = { ...args, ...extraQueryParams, ...runnableInputValues }

	// TODO: Review
	function setStaticInputsToArgs() {
		Object.entries(inputs).forEach(([key, value]) => {
			if (value.type === 'static') {
				args[key] = value.value
			}
		})

		args = args
	}

	$: inputs && setStaticInputsToArgs()

	function argMergedArgsValid(mergedArgs: Record<string, any>) {
		if (
			Object.keys(inputs).filter((k) => inputs[k].type !== 'user').length !==
			Object.keys(runnableInputValues).length
		) {
			return false
		}

		const areAllArgsValid = Object.values(mergedArgs).every(
			(arg) => arg !== undefined && arg !== null
		)

		if (areAllArgsValid && autoRefresh) {
			executeComponent()
		}

		return areAllArgsValid
	}

	$: isValid = argMergedArgsValid(mergedArgs)

	// Test job internal state
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<Array<any>>
		loading: Output<boolean>
	}

	async function loadSchemaFromTriggerable(
		workspace: string,
		path: string,
		runType: 'script' | 'flow'
	) {
		schema = await loadSchema(workspace, path, runType)
	}

	// Only loads the schema
	$: if ($workspaceStore && path && runType) {
		// Remote schema needs to be loaded
		loadSchemaFromTriggerable($workspaceStore, path, runType)
	} else if (inlineScriptName && $app.inlineScripts[inlineScriptName]) {
		// Inline scripts directly provide the schema
		schema = $app.inlineScripts[inlineScriptName].schema
	}

	// When the schema is loaded, we need to update the inputs spec
	// in order to render the inputs the component panel
	$: if (schema && Object.keys(schema.properties).length !== Object.keys(inputs).length) {
		let schemaWithoutExtraQueries: Schema = JSON.parse(JSON.stringify(schema))

		// Remove extra query params from the schema, which are not directly configurable by the user
		Object.keys(extraQueryParams).forEach((key) => {
			delete schemaWithoutExtraQueries.properties[key]
		})

		inputs = schemaToInputsSpec(schemaWithoutExtraQueries)
	}

	let schemaStripped: Schema | undefined = undefined

	function stripSchema(schema: Schema) {
		schemaStripped = JSON.parse(JSON.stringify(schema))

		// Remove hidden static inputs
		Object.keys(inputs).forEach((key: string) => {
			const input = inputs[key]

			if (input.type === 'static' && !input.visible && schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}

			if (input.type === 'output' && schemaStripped !== undefined) {
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

	$: schema && stripSchema(schema)

	$: disabledArgs = Object.keys(inputs).reduce(
		(disabledArgsAccumulator: string[], inputName: string) => {
			if (inputs[inputName].type === 'static') {
				disabledArgsAccumulator = [...disabledArgsAccumulator, inputName]
			}
			return disabledArgsAccumulator
		},
		[]
	)

	async function executeComponent() {
		outputs?.loading.set(true)

		await testJobLoader?.abstractRun(() => {
			const requestBody = {
				args: mergedArgs,
				force_viewer_static_fields: {}
			}

			if (inlineScriptName && $app.inlineScripts[inlineScriptName]) {
				requestBody['raw_code'] = {
					content: $app.inlineScripts[inlineScriptName].content,
					language: $app.inlineScripts[inlineScriptName].language,
					path: $app.inlineScripts[inlineScriptName].path
				}
			} else if (path && runType) {
				requestBody['path'] = `${runType}/${path}`
			}

			return AppService.executeComponent({
				workspace: $workspaceStore!,
				path: pagePath,
				requestBody
			})
		})
	}

	export function runComponent() {
		executeComponent()
	}
</script>

{#each Object.keys(inputs) as key}
	<InputValue input={inputs[key]} bind:value={runnableInputValues[key]} />
{/each}

<TestJobLoader
	on:done={() => {
		if (testJob) {
			outputs.result.set(testJob?.result)
			outputs?.loading.set(false)

			result = testJob?.result
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

{#if schemaStripped !== undefined}
	<SchemaForm schema={schemaStripped} bind:args {isValid} {disabledArgs} shouldHideNoInputs />
{/if}

{#if autoRefresh === true}
	{#if isValid}
		<Button size="xs" color="dark" on:click={() => executeComponent()} disabled={!isValid}>
			<div>
				{Object.keys(args).length > 0 ? 'Submit' : 'Refresh'}
				{#if testIsLoading}
					<Icon data={faArrowsRotate} class="animate-spin ml-2" scale={0.8} />
				{/if}
			</div>
		</Button>
		<slot />
	{:else}
		<Alert type="warning" size="xs" class="mt-2" title="Missing inputs">
			Please fill in all the inputs
		</Alert>
	{/if}
{:else}
	<slot />
{/if}
