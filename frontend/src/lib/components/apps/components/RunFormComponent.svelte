<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { CompletedJob, FlowService, Job, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faArrowsRotate, faFile } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { Output } from '../rx'
	import type { AppEditorContext, InputsSpec } from '../types'

	export let runType: 'script' | 'flow'
	export let path: string
	export let id: string

	export let inputs: {
		runInputs: InputsSpec
	}

	export let hidden: string[] = []
	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export let schema: Schema | undefined = undefined
	let schemaClone: Schema | undefined = undefined

	export const staticOutputs = ['loading', 'result']

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<any>
		loading: Output<boolean>
	}

	async function loadSchema(workspace: string) {
		if (runType === 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace,
				path
			})
			schema = script.schema
		} else if (runType === 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace,
				path
			})

			schema = flow.schema
		}

		schemaClone = JSON.parse(JSON.stringify(schema))
	}

	function mapInput(schema: Schema) {
		Object.keys(inputs.runInputs).forEach((argName) => {
			const arg = inputs.runInputs[argName]

			if (hidden.includes(argName)) {
				delete schema.properties[argName]
			}

			if (arg.type === 'static' && schema.properties[argName]) {
				schema.properties[argName].default = arg.value
			}
		})
	}

	$: if ($workspaceStore) {
		loadSchema($workspaceStore)
	}

	$: schemaClone && mapInput(schemaClone)

	let x = buildArgs(inputs.runInputs)

	function buildArgs(args: InputsSpec) {
		return Object.keys(args)
			.filter((x) => hidden.includes(x))
			.reduce((previousValue: Record<string, any>, currentValue: string) => {
				const arg = args[currentValue]

				if (arg.type === 'static') {
					previousValue[currentValue] = arg.value
				}
				return previousValue
			}, {})
	}

	function extractHiddenParamsFromSchemas(schema: Schema | undefined) {
		if (schema) {
			return Object.keys(inputs.runInputs).reduce(
				(previousValue: Record<string, any>, currentValue: string) => {
					const arg = inputs.runInputs[currentValue]

					if (arg.type === 'static') {
						previousValue[currentValue] = arg.value
					}
					return previousValue
				},
				{}
			)
		} else {
			return {}
		}
	}

	let isValid = true
	let testIsLoading = false
	let testJob: CompletedJob | undefined = undefined

	let testJobLoader: TestJobLoader | undefined = undefined
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
	<SchemaForm bind:schema={schemaClone} bind:args={x} bind:isValid />
	<Button
		size="xs"
		color="dark"
		variant="border"
		on:click={() => {
			const k = extractHiddenParamsFromSchemas(schema)

			testJobLoader?.runScriptByPath(path, {
				...k,
				...x
			})
		}}
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
