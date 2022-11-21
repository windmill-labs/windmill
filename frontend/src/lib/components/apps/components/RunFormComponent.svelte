<script lang="ts">
	import { page } from '$app/stores'
	import type { Schema } from '$lib/common'
	import { Button } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faArrowsRotate, faFile } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { Output } from '../rx'
	import type { AppEditorContext, InputsSpec } from '../types'
	import { buildArgs, loadSchema, schemaToInputsSpec } from '../utils'

	// Component props
	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined

	export const staticOutputs = ['loading', 'result']

	const { worldStore, app } = getContext<AppEditorContext>('AppEditorContext')
	let pagePath = $page.params.path

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<any>
		loading: Output<boolean>
	}

	// Local state
	let args: Record<string, any> = {}
	let schema: Schema | undefined = undefined
	let schemaClone: Schema | undefined = undefined

	let isValid = true
	let testIsLoading = false
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: if ($workspaceStore && path && runType) {
		loadSchemaFromTriggerable($workspaceStore, path, runType)
	}

	$: if (inlineScriptName) {
		schema = $app.inlineScripts[inlineScriptName].schema
		reloadSchemaAndArgs()
	}

	$: if (inputs && schema !== undefined) {
		if (Object.keys(schema.properties).length !== Object.keys(inputs).length) {
			inputs = schemaToInputsSpec(schema)
		}

		reloadSchemaAndArgs()
	}

	// Load once
	async function loadSchemaFromTriggerable(
		workspace: string,
		path: string,
		runType: 'script' | 'flow'
	) {
		schema = await loadSchema(workspace, path, runType)
		args = buildArgs(inputs, schema)
	}

	async function reloadSchemaAndArgs() {
		schemaClone = JSON.parse(JSON.stringify(schema))

		if (schemaClone !== undefined) {
			args = buildArgs(inputs, schemaClone)

			Object.keys(schemaClone.properties).forEach((propKey) => {
				if (!Object.keys(args).includes(propKey)) {
					delete schemaClone!.properties[propKey]
				}
			})
		}
	}

	$: disabledArgs = Object.keys(inputs).reduce((a: string[], c: string) => {
		if (inputs[c].type === 'static') {
			a = [...a, c]
		}
		return a
	}, [])

	async function executeComponent() {
		await testJobLoader?.abstractRun(() => {
			const requestBody = {
				args,
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

		outputs?.loading.set(true)
	}
</script>

<TestJobLoader
	on:done={() => {
		if (testJob) {
			outputs?.result.set(testJob?.result)
			outputs?.loading.set(false)
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

{#if schemaClone !== undefined}
	<SchemaForm schema={schemaClone} bind:args bind:isValid {disabledArgs} />
	<Button
		size="xs"
		color="dark"
		variant="border"
		on:click={() => executeComponent()}
		startIcon={{ icon: faFile }}
		disabled={!isValid}
	>
		<div>
			Submit
			{#if testIsLoading}
				<Icon data={faArrowsRotate} class="animate-spin ml-2" scale={0.8} />
			{/if}
		</div>
	</Button>
{/if}
