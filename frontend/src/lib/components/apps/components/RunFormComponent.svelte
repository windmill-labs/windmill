<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { CompletedJob, FlowService, Job, ScriptService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faArrowsRotate, faFile } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { Output } from '../rx'
	import type { App, AppEditorContext, InputsSpec } from '../types'
	import { isAppInputTransformHidden, schemaToInputsSpec } from '../utils'

	// Component props
	export let id: string
	export let inputs: InputsSpec

	// Extra props
	export const staticOutputs = ['loading', 'result']
	const { worldStore, app } = getContext<AppEditorContext>('AppEditorContext')

	let schema: Schema | undefined = undefined
	let isValid = true
	let testIsLoading = false
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: args = inputsSpecToSchemaArgs(inputs, $app)
	$: triggerable = $app.policy?.triggerables[id]

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<any>
		loading: Output<boolean>
	}

	$: if ($workspaceStore) {
		loadSchema($workspaceStore)
	}

	$: schema && app && deleteHiddenArgFromSchema(schema, $app)

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
	}

	// DONE
	function deleteHiddenArgFromSchema(schema: Schema, app: App) {
		if (!app.policy.triggerables[id]) {
			throw new Error(`Triggerable ${id} not found`)
		}

		const { staticFields } = app.policy.triggerables[id]

		Object.keys(staticFields).forEach((argName) => {
			const input = inputs[argName]

			if (isAppInputTransformHidden(input)) {
				delete schema.properties[argName]
			}
		})
	}

	function inputsSpecToSchemaArgs(
		args: InputsSpec,
		app: App
	): Record<string, InputTransform | any> {
		if (!app.policy.triggerables[id]) {
			throw new Error(`Triggerable ${id} not found`)
		}

		const { staticFields } = app.policy.triggerables[id]

		return Object.keys(args).reduce(
			(previousValue: Record<string, InputTransform>, currentValue: string) => {
				const arg = args[currentValue]

				if (arg.type === 'static') {
					previousValue[currentValue] = staticFields[currentValue]
				}
				return previousValue
			},
			{}
		)
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

{#if schema !== undefined}
	<SchemaForm bind:schema bind:args bind:isValid />
	<Button
		size="xs"
		color="dark"
		variant="border"
		on:click={() => {
			testJobLoader?.runScriptByPath(triggerable?.path, args)
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
