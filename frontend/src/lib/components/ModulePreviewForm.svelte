<script lang="ts">
	import type { Schema } from '$lib/common'
	import ResizeTransitionWrapper from './common/ResizeTransitionWrapper.svelte'
	import { allTrue } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { evalValue } from './flows/utils.svelte'
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

	const { stepsInputArgs, flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let inputCheck: { [id: string]: boolean } = $state({})
	$effect(() => {
		isValid = allTrue(inputCheck) ?? false
	})

	let keys: string[] = $state([])
	$effect(() => {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			untrack(() => stepsInputArgs?.removeExtraKey(mod.id, keys))
		}
	})

	function plugIt(argName: string) {
		stepsInputArgs?.setEvaluatedStepArg(
			mod.id,
			argName,
			$state.snapshot(evalValue(argName, mod, pickableProperties, true))
		)
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

	let initialized = $state(false)

	$effect.pre(() => {
		if (!initialized) {
			if (stepsInputArgs) {
				stepsInputArgs?.updateStepArgs(mod.id, flowStateStore.val, flowStore?.val, previewArgs?.val)
				initialized = true
			}
		}
	})
</script>

<div class="w-full pt-2" data-popover>
	{#if initialized}
		{#if keys.length > 0}
			{#each keys as argName, i (argName)}
				{#if Object.keys(schema.properties ?? {}).includes(argName)}
					<ResizeTransitionWrapper
						vertical
						class={twMerge(
							'flex gap-2',
							animateArg === argName && 'animate-pulse ring-2 ring-offset-2 ring-blue-500 rounded'
						)}
						innerClass="w-full"
						outerDivProps={{ 'data-arg': argName }}
					>
						{#if schema?.properties?.[argName]}
							<ArgInput
								{resourceTypes}
								minW={false}
								autofocus={autofocus && !focusArg && i == 0}
								label={argName}
								description={schema.properties[argName].description}
								bind:value={
									() => stepsInputArgs?.getStepInputArgs(mod.id, argName),
									(v) => stepsInputArgs?.setStepInputArgs(mod.id, argName, v)
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
							>
								{#snippet fieldHeaderActions()}
									{#if stepsInputArgs?.isArgManuallySet(mod.id, argName)}
										<Button
											on:click={() => {
												plugIt(argName)
											}}
											size="xs2"
											variant="contained"
											color="light"
											title="Re-evaluate input step"><RefreshCw size={12} /></Button
										>
									{/if}
								{/snippet}
							</ArgInput>
						{/if}
					</ResizeTransitionWrapper>
				{/if}
			{/each}
		{/if}
	{:else}
		<div class="text-center text-sm text-primary"> Loading test step arguments... </div>
	{/if}
</div>
