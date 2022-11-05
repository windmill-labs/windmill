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
	import { schemaToInputsSpec } from '../editor/settingsPanel/utils'
	import type { Output } from '../rx'
	import type { AppEditorContext, InputsSpec } from '../types'

	// Component props
	export let id: string
	export let inputs: InputsSpec
	export let hidden: string[] = []

	// Extra props
	let schema: Schema | undefined = undefined
	export const staticOutputs = ['loading', 'result']

	const { worldStore, app } = getContext<AppEditorContext>('AppEditorContext')

	let schemaClone: Schema | undefined = undefined
	let isValid = true
	let testIsLoading = false
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined
	let x = buildArgs(inputs)
	$: triggerable = $app.policy?.triggerables[id]

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<any>
		loading: Output<boolean>
	}

	$: if ($workspaceStore) {
		loadSchema($workspaceStore)
	}

	$: schemaClone && mapInput(schemaClone)

	async function loadSchema(workspace: string) {
		if (triggerable?.type === 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace,
				path: triggerable.path
			})

			schema = script.schema

			inputs = schemaToInputsSpec(schema)
		} else if (triggerable?.type === 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace,
				path: triggerable.path
			})

			schema = flow.schema
			inputs = schemaToInputsSpec(schema)
		}

		schemaClone = JSON.parse(JSON.stringify(schema))
	}

	function mapInput(schema: Schema) {
		Object.keys(inputs).forEach((argName) => {
			const arg = inputs[argName]

			if (hidden.includes(argName)) {
				delete schema.properties[argName]
			}

			if (arg.type === 'static' && schema.properties[argName]) {
				schema.properties[argName].default = $app.policy?.triggerables[id]?.staticFields[argName]
			}
		})
	}

	function buildArgs(args: InputsSpec) {
		return Object.keys(args)
			.filter((x) => hidden.includes(x))
			.reduce((previousValue: Record<string, any>, currentValue: string) => {
				const arg = args[currentValue]

				if (arg.type === 'static') {
					previousValue[currentValue] = $app.policy?.triggerables[id]?.staticFields[currentValue]
				}
				return previousValue
			}, {})
	}

	function extractHiddenParamsFromSchemas(schema: Schema | undefined) {
		if (schema) {
			return Object.keys(inputs).reduce(
				(previousValue: Record<string, any>, currentValue: string) => {
					const arg = inputs[currentValue]

					if (arg.type === 'static') {
						previousValue[currentValue] = $app.policy?.triggerables[id]?.staticFields[currentValue]
					}
					return previousValue
				},
				{}
			)
		} else {
			return {}
		}
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
	<SchemaForm bind:schema={schemaClone} bind:args={x} bind:isValid />
	<Button
		size="xs"
		color="dark"
		variant="border"
		on:click={() => {
			const k = extractHiddenParamsFromSchemas(schema)

			testJobLoader?.runScriptByPath(triggerable?.path, {
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
