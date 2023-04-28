<script lang="ts">
	import type { Schema } from '$lib/common'

	import { allTrue } from '$lib/utils'
	import { Plug } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { evalValue } from './flows/utils'
	import type { FlowModule } from '$lib/gen'
	import type { PickableProperties } from './flows/previousResults'

	export let schema: Schema
	export let args: Record<string, any> = {}
	export let mod: FlowModule
	export let pickableProperties: PickableProperties | undefined

	export let isValid: boolean = true
	export let autofocus = false

	const { testStepStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	let keys: string[] = []
	$: {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			removeExtraKey()
		}
	}

	function plugIt(argName: string) {
		args[argName] = evalValue(argName, mod, testStepStore, pickableProperties, true)
	}
</script>

<div class="w-full pt-4">
	{#if keys.length > 0}
		{#each keys as argName, i (argName)}
			{#if Object.keys(schema.properties ?? {}).includes(argName)}
				<div class="flex gap-2 items-center">
					{#if typeof args == 'object' && schema?.properties[argName]}
						<ArgInput
							minW={false}
							autofocus={i == 0 && autofocus}
							label={argName}
							description={schema.properties[argName].description}
							bind:value={args[argName]}
							type={schema.properties[argName].type}
							required={schema.required.includes(argName)}
							pattern={schema.properties[argName].pattern}
							bind:valid={inputCheck[argName]}
							defaultValue={schema.properties[argName].default}
							enum_={schema.properties[argName].enum}
							format={schema.properties[argName].format}
							contentEncoding={schema.properties[argName].contentEncoding}
							properties={schema.properties[argName].properties}
							itemsType={schema.properties[argName].items}
							extra={schema.properties[argName]}
						/>
					{/if}
					<div>
						<Button
							on:click={() => plugIt(argName)}
							size="sm"
							variant="border"
							color="light"
							title="Eval input component"><Plug size={14} /></Button
						>
					</div>
				</div>
			{/if}
		{/each}
	{/if}
</div>
