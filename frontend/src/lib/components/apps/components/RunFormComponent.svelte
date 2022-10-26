<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import {
		FlowService,
		Job,
		JobService,
		Preview,
		ScriptService,
		type InputTransform
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faFile } from '@fortawesome/free-solid-svg-icons'
	import type { Inputs } from '../types'

	export let runType: 'script' | 'flow'
	export let path: string
	export let args: Inputs
	export let hidden: string[] = []

	let schema: Schema | undefined = undefined
	let schemaClone: Schema | undefined = undefined

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
		Object.keys(args).forEach((argName) => {
			const arg = args[argName]

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

	let x = buildArgs(args)

	function buildArgs(args: Inputs) {
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
			return Object.keys(args).reduce(
				(previousValue: Record<string, any>, currentValue: string) => {
					const arg = args[currentValue]

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
</script>

{#if schemaClone !== undefined}
	<SchemaForm bind:schema={schemaClone} bind:args={x} bind:isValid />
	<Button
		size="xs"
		color="dark"
		variant="border"
		on:click={() => {
			const k = extractHiddenParamsFromSchemas(schema)
			alert(
				JSON.stringify({
					...k,
					...x
				})
			)
		}}
		startIcon={{ icon: faFile }}
		disabled={!isValid}
	>
		Submit
	</Button>
{/if}
