<script lang="ts">
	import type { Schema } from '$lib/common'

	import { allTrue } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { evalValue } from './flows/utils'
	import type { FlowModule } from '$lib/gen'
	import type { PickableProperties } from './flows/previousResults'
	import type SimpleEditor from './SimpleEditor.svelte'
	import { getResourceTypes } from './resourceTypesStore'

	interface Props {
		schema: Schema | { properties?: Record<string, any>; required?: string[] }
		args?: Record<string, any>
		mod: FlowModule
		pickableProperties: PickableProperties | undefined
		isValid?: boolean
		autofocus?: boolean
	}

	let {
		schema,
		args = $bindable({}),
		mod,
		pickableProperties,
		isValid = $bindable(true),
		autofocus = false
	}: Props = $props()

	const { testStepStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let inputCheck: { [id: string]: boolean } = $state({})
	$effect(() => {
		isValid = allTrue(inputCheck) ?? false
	})

	$effect(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	let keys: string[] = $state([])
	$effect(() => {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			untrack(() => removeExtraKey())
		}
	})

	function plugIt(argName: string) {
		args[argName] = structuredClone(
			$state.snapshot(evalValue(argName, mod, testStepStore, pickableProperties, true))
		)
		try {
			editor?.[argName]?.setCode(JSON.stringify(args[argName], null, 4))
		} catch {
			//ignore
		}
	}

	let editor: Record<string, SimpleEditor | undefined> = $state({})

	let resourceTypes: string[] | undefined = $state(undefined)

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	loadResourceTypes()
</script>

<div class="w-full pt-2">
	{#if keys.length > 0}
		{#each keys as argName, i (argName)}
			{#if Object.keys(schema.properties ?? {}).includes(argName)}
				<div class="flex gap-2">
					{#if typeof args == 'object' && schema?.properties?.[argName]}
						<ArgInput
							{resourceTypes}
							minW={false}
							autofocus={i == 0 && autofocus}
							label={argName}
							description={schema.properties[argName].description}
							bind:value={args[argName]}
							type={schema.properties[argName].type}
							oneOf={schema.properties[argName].oneOf}
							required={schema?.required?.includes(argName)}
							pattern={schema.properties[argName].pattern}
							bind:editor={editor[argName]}
							bind:valid={inputCheck[argName]}
							defaultValue={schema.properties[argName].default}
							enum_={schema.properties[argName].enum}
							format={schema.properties[argName].format}
							contentEncoding={schema.properties[argName].contentEncoding}
							properties={schema.properties[argName].properties}
							nestedRequired={schema.properties[argName].required}
							itemsType={schema.properties[argName].items}
							extra={schema.properties[argName]}
							nullable={schema.properties[argName].nullable}
							title={schema.properties[argName].title}
							placeholder={schema.properties[argName].placeholder}
						/>
					{/if}
					<div class="pt-6 mt-0.5">
						<Button
							on:click={() => plugIt(argName)}
							size="sm"
							variant="border"
							color="light"
							title="Re-evaluate input step"><RefreshCw size={14} /></Button
						>
					</div>
				</div>
			{/if}
		{/each}
	{/if}
</div>
