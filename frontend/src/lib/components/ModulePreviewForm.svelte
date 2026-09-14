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
	import { workspaceStore } from '$lib/stores'
	import { AGENT_FIELDS, initialVisibleAgentFields } from './flows/agentFormFields'

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

	const { stepsInputArgs, flowStateStore, flowStore, previewArgs, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

	let inputCheck: { [id: string]: boolean } = $state({})
	$effect(() => {
		isValid = allTrue(inputCheck) ?? false
	})

	/** An agent asks for the same fields here that its own form shows: a setting the step leaves
	 *  unset is not something a run needs told, and listing all eleven buries the message under the
	 *  configuration. What the step configures stays, as it does on any other step. A schema key the
	 *  field registry doesn't know is kept, so a new one is never silently dropped. A run input is
	 *  kept whatever the step holds: this form has no add-field control, so hiding one would leave
	 *  no way at all to supply it. */
	let schemaKeys = $derived(Object.keys(schema?.properties ?? {}))

	let visibleKeys = $derived.by(() => {
		const all = schemaKeys
		if ((mod.value as { type?: string })?.type !== 'aiagent') return all
		const transforms = (mod.value as { input_transforms?: Record<string, unknown> })
			?.input_transforms
		const visible = initialVisibleAgentFields(transforms, schema?.properties)
		const known = new Set(AGENT_FIELDS.filter((f) => !f.runInput).map((f) => f.key))
		return all.filter((key) => !known.has(key) || visible.has(key))
	})

	let keys: string[] = $state([])
	$effect(() => {
		let lkeys = visibleKeys
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			// Pruned against the schema rather than against what is shown. What a run was given for a
			// field lives only here, so dropping it when the field merely stops being displayed would
			// discard it: an agent hides the settings its step leaves unset, and clearing one in the
			// Inputs tab hides it.
			untrack(() => stepsInputArgs?.removeExtraKey(mod.id, schemaKeys))
		}
	})

	/** Whether re-evaluating has anything to restore. A field the step configures nothing for
	 *  evaluates to blank, so the control would only clear what was typed to run with. */
	function hasConfiguredInput(argName: string): boolean {
		const transform = (mod.value as any)?.input_transforms?.[argName]
		if (!transform) return false
		return transform.type === 'javascript' ? !!transform.expr : transform.value !== undefined
	}

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
								workspace={opWs}
							>
								{#snippet fieldHeaderActions()}
									{#if stepsInputArgs?.isArgManuallySet(mod.id, argName) && hasConfiguredInput(argName)}
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
