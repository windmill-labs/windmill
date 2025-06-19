<script lang="ts">
	import type { Schema } from '$lib/common'

	import { allTrue } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import { getContext, onMount, untrack } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { evalValue } from './flows/utils'
	import type { FlowModule } from '$lib/gen'
	import type { PickableProperties } from './flows/previousResults'
	import type SimpleEditor from './SimpleEditor.svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		schema: Schema | { properties?: Record<string, any>; required?: string[] }
		mod: FlowModule
		pickableProperties: PickableProperties | undefined
		isValid?: boolean
		autofocus?: boolean
		focusArg?: string
	}

	let {
		schema,
		mod,
		pickableProperties,
		isValid = $bindable(true),
		autofocus = false,
		focusArg = undefined
	}: Props = $props()

	const { testSteps, flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let inputCheck: { [id: string]: boolean } = $state({})
	$effect(() => {
		isValid = allTrue(inputCheck) ?? false
	})

	$effect(() => {
		if (
			testSteps?.getStepArgs(mod.id) == undefined ||
			typeof testSteps?.getStepArgs(mod.id) !== 'object'
		) {
			testSteps?.setStepArgs(mod.id, {})
		}
	})

	let keys: string[] = $state([])
	$effect(() => {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			untrack(() => testSteps?.removeExtraKey(mod.id, keys))
		}
	})

	function plugIt(argName: string) {
		testSteps?.setStepArg(
			mod.id,
			argName,
			structuredClone(
				$state.snapshot(
					evalValue(argName, mod, testSteps.getStepArgs(mod.id) ?? {}, pickableProperties, true)
				)
			)
		)
		testSteps?.resetArgManually(mod.id, argName)
	}

	let editor: Record<string, SimpleEditor | undefined> = $state({})

	// Animation and highlighting for focusArg
	let animateArg: string | undefined = $state(undefined)
	$effect(() => {
		if (focusArg) {
			// Add a slight delay to ensure the form is rendered
			setTimeout(() => {
				const argElement = document.querySelector(`[data-arg="${focusArg}"]`)
				if (argElement) {
					// Add highlight animation
					animateArg = focusArg
					argElement.scrollIntoView({ behavior: 'smooth', block: 'center' })

					// Focus the input if it exists
					const input = argElement.querySelector('input, textarea, select') as
						| HTMLInputElement
						| HTMLTextAreaElement
						| HTMLSelectElement
						| null
					if (input) {
						input.focus()
					}

					// Remove highlight after animation
					setTimeout(() => {
						animateArg = undefined
					}, 2000)
				}
			}, 200)
		}
	})

	let resourceTypes: string[] | undefined = $state(undefined)

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	loadResourceTypes()

	onMount(() => {
		testSteps?.updateStepArgs(mod.id, $flowStateStore, flowStore?.val, previewArgs?.val)
	})
</script>

<div class="w-full pt-2" data-popover>
	{#if keys.length > 0}
		{#each keys as argName, i (argName)}
			{#if Object.keys(schema.properties ?? {}).includes(argName)}
				<div
					class={twMerge(
						'flex gap-2',
						animateArg === argName && 'animate-pulse ring-2 ring-offset-2 ring-blue-500 rounded'
					)}
					data-arg={argName}
				>
					{#if typeof testSteps?.getStepArgs(mod.id) == 'object' && schema?.properties?.[argName]}
						<ArgInput
							{resourceTypes}
							minW={false}
							autofocus={autofocus && !focusArg && i == 0}
							label={argName}
							description={schema.properties[argName].description}
							bind:value={
								() => testSteps?.getStepArg(mod.id, argName),
								(v) => testSteps?.setStepArg(mod.id, argName, v, true)
							}
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
					{#if testSteps?.manuallySetArgs[mod.id]?.[argName]}
						<div class="pt-6 mt-0.5">
							<Button
								on:click={() => {
									plugIt(argName)
								}}
								size="sm"
								variant="border"
								color="light"
								title="Re-evaluate input step"><RefreshCw size={14} /></Button
							>
						</div>
					{/if}
				</div>
			{/if}
		{/each}
	{/if}
</div>
