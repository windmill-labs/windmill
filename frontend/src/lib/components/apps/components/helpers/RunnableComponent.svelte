<script lang="ts">
	import { page } from '$app/stores'
	import type { Schema } from '$lib/common'
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
	import { buildArgs, loadSchema, schemaToInputsSpec } from '../../utils'
	import InputValue from './InputValue.svelte'

	// Component props
	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let shouldTick: number | undefined = undefined
	export let result: any = undefined

	const { app, worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let pagePath = $page.params.path

	// Local state
	let args: Record<string, any> = {}
	let schema: Schema | undefined = undefined
	let isValid = true
	let testIsLoading = false

	// Test job internal state
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<Array<any>>
		loading: Output<boolean>
	}

	/**
	 * Args are built from 3 sources:
	 * 1. The inputs spec ( )
	 * 2. The schema input transform with user submitted values§
	 * 3. The extra query params
	 */

	async function loadSchemaFromTriggerable(
		workspace: string,
		path: string,
		runType: 'script' | 'flow'
	) {
		schema = await loadSchema(workspace, path, runType)
	}

	// Only loads the schema
	$: if ($workspaceStore && path && runType) {
		loadSchemaFromTriggerable($workspaceStore, path, runType)
	} else if (inlineScriptName && $app.inlineScripts[inlineScriptName]) {
		schema = $app.inlineScripts[inlineScriptName].schema
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

	$: disabledArgs = Object.keys(inputs).reduce((a: string[], c: string) => {
		if (inputs[c].type === 'static') {
			a = [...a, c]
		}
		return a
	}, [])

	async function executeComponent() {
		await testJobLoader?.abstractRun(() => {
			const requestBody = {
				args: {
					...args,
					...extraQueryParams
				},
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
</script>

{#each Object.keys(inputs) as key}
	<InputValue input={inputs[key]} bind:value={args[key]} />
{/each}

<TestJobLoader
	on:done={() => {
		if (testJob) {
			outputs.result.set(testJob?.result)
			result = testJob?.result
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

{#if schemaStripped !== undefined}
	<SchemaForm schema={schemaStripped} bind:args {isValid} {disabledArgs} />
{/if}

{#if shouldTick === undefined}
	<Button size="xs" color="dark" on:click={() => executeComponent()} disabled={!isValid}>
		<div>
			{Object.keys(args).length > 0 ? 'Submit' : 'Refresh'}
			{#if testIsLoading}
				<Icon data={faArrowsRotate} class="animate-spin ml-2" scale={0.8} />
			{/if}
		</div>
	</Button>
{/if}
<slot />
