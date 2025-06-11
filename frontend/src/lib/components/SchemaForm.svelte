<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { Schema } from '$lib/common'
	import { VariableService, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue, computeShow } from '$lib/utils'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import { Plus } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { dragHandleZone, type Options as DndOptions } from '@windmill-labs/svelte-dnd-action'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	import type { ComponentCustomCSS } from './apps/types'

	interface Props {
		schema: Schema | any
		hiddenArgs?: string[]
		schemaFieldTooltip?: Record<string, string>
		args?: Record<string, any>
		disabledArgs?: string[]
		disabled?: boolean
		isValid?: boolean
		autofocus?: boolean
		defaultValues?: Record<string, any>
		shouldHideNoInputs?: boolean
		compact?: boolean
		linkedSecret?: string | undefined
		linkedSecretCandidates?: string[] | undefined
		noVariablePicker?: boolean
		flexWrap?: boolean
		noDelete?: boolean
		prettifyHeader?: boolean
		disablePortal?: boolean
		showSchemaExplorer?: boolean
		showReset?: boolean
		onlyMaskPassword?: boolean
		dndConfig?: DndOptions | undefined
		items?: { id: string; value: string }[] | undefined
		helperScript?:
			| { type: 'inline'; path?: string; lang: Script['language']; code: string }
			| { type: 'hash'; hash: string }
			| undefined
		lightHeader?: boolean
		diff?: Record<string, SchemaDiff>
		nestedParent?: { label: string; nestedParent: any | undefined } | undefined
		shouldDispatchChanges?: boolean
		nestedClasses?: string
		dynamicEnums?: Record<string, any>
		largeGap?: boolean
		css?: ComponentCustomCSS<'schemaformcomponent'> | undefined
		displayType?: boolean
		appPath?: string | undefined
		className?: string
		computeS3ForceViewerPolicies?:
			| (() =>
					| {
							allowed_resources: string[]
							allow_user_resources: boolean
							allow_workspace_resource: boolean
							file_key_regex: string
					  }
					| undefined)
			| undefined
		workspace?: string | undefined
		actions?: import('svelte').Snippet
	}

	let {
		schema = $bindable(),
		hiddenArgs = [],
		schemaFieldTooltip = {},
		args = $bindable(undefined),
		disabledArgs = [],
		disabled = false,
		isValid = $bindable(true),
		autofocus = false,
		defaultValues = {},
		shouldHideNoInputs = false,
		compact = false,
		linkedSecret = $bindable(undefined),
		linkedSecretCandidates = undefined,
		noVariablePicker = false,
		flexWrap = false,
		noDelete = false,
		prettifyHeader = false,
		disablePortal = false,
		showSchemaExplorer = false,
		showReset = false,
		onlyMaskPassword = false,
		dndConfig = undefined,
		items = undefined,
		helperScript = undefined,
		lightHeader = false,
		diff = {},
		nestedParent = undefined,
		shouldDispatchChanges = false,
		nestedClasses = '',
		dynamicEnums = {},
		largeGap = false,
		css = undefined,
		displayType = true,
		appPath = undefined,
		className = '',
		computeS3ForceViewerPolicies = undefined,
		workspace = undefined,
		actions
	}: Props = $props()

	$effect.pre(() => {
		if (args == undefined) {
			args = {}
		}
	})

	const dispatch = createEventDispatcher()

	let inputCheck: { [id: string]: boolean } = $state({})

	export function setDefaults() {
		const nargs = structuredClone($state.snapshot(defaultValues))

		Object.keys(schema?.properties ?? {}).forEach((key) => {
			if (schema?.properties[key].default != undefined && args && args[key] == undefined) {
				let value = schema?.properties[key].default
				nargs[key] = value === 'object' ? structuredClone($state.snapshot(value)) : value
			}
		})
		args = nargs
	}

	let keys: string[] = $state([])

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key) && args) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	let pickForField: string | undefined = $state()
	let itemPicker: ItemPicker | undefined = $state(undefined)
	let variableEditor: VariableEditor | undefined = $state(undefined)

	let resourceTypes: string[] | undefined = $state(undefined)

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	loadResourceTypes()

	function hasExtraKeys() {
		return Object.keys(args ?? {}).some((x) => !keys.includes(x))
	}

	function reorder() {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (!deepEqual(schema?.order, lkeys) || !deepEqual(keys, lkeys)) {
			if (schema?.order && Array.isArray(schema.order)) {
				const n = {}

				;(schema.order as string[]).forEach((x) => {
					if (schema.properties && schema.properties[x] != undefined) {
						n[x] = schema.properties[x]
					}
				})

				Object.keys(schema.properties ?? {})
					.filter((x) => !schema.order?.includes(x))
					.forEach((x) => {
						n[x] = schema.properties[x]
					})
				schema.properties = n
			}
			let nkeys = Object.keys(schema.properties ?? {})

			if (!deepEqual(keys, nkeys)) {
				keys = nkeys
				dispatch('change')
			}
		}

		if (!noDelete && hasExtraKeys()) {
			removeExtraKey()
		}
	}

	let hidden: Record<string, boolean> = $state({})
	let fields = $derived(items ?? keys.map((x) => ({ id: x, value: x })))

	function handleHiddenFields(schema: Schema | any, args: Record<string, any>) {
		for (const x of fields) {
			if (schema?.properties?.[x.value]?.showExpr) {
				if (computeShow(x.value, schema.properties?.[x.value]?.showExpr, args)) {
					hidden[x.value] = false
				} else if (!hidden[x.value]) {
					hidden[x.value] = true
					// remove arg (important: will not trigger a re-render)
					delete args[x.value]
					// make sure it's made valid
					inputCheck[x.value] = true
				}
			}
		}
	}

	$effect(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})
	$effect(() => {
		const newKeys = Array.isArray(schema?.order)
			? schema?.order
			: Object.keys(schema?.properties ?? {})
		if (!deepEqual(keys, newKeys)) {
			keys = newKeys
		}
	})
	$effect(() => {
		schema && (untrack(() => reorder()), (hidden = {}))
	})
	$effect(() => {
		handleHiddenFields(schema, untrack(() => args) ?? {})
	})
	$effect(() => {
		isValid = allTrue(inputCheck ?? {})
	})
	const actions_render = $derived(actions)
</script>

<!-- {JSON.stringify(args)} -->
{#if showReset}
	<div class="flex flex-row-reverse w-full">
		<Button size="xs" color="light" on:click={() => setDefaults()}>
			Reset args to runnable's defaults
		</Button>
	</div>
{/if}

<div
	class="w-full {className} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''} {nestedClasses}"
	use:dragHandleZone={dndConfig ?? { items: [], dragDisabled: true }}
	onfinalize={bubble('finalize')}
	onconsider={bubble('consider')}
>
	{#if keys.length > 0 && args}
		{#each fields as item, i (item.id)}
			{@const argName = item.value}
			<div
				class={typeof diff[argName] === 'object' && diff[argName].diff !== 'same'
					? 'bg-red-300 dark:bg-red-800 rounded-md'
					: ''}
			>
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				{#if !hiddenArgs.includes(argName) && keys.includes(argName)}
					{#if typeof diff[argName] === 'object' && diff[argName].oldSchema}
						{@const formerProperty = diff[argName].oldSchema}
						<div class="px-2">
							<ArgInput
								{disablePortal}
								{resourceTypes}
								{prettifyHeader}
								autofocus={i == 0 && autofocus ? true : null}
								label={argName}
								description={formerProperty?.description}
								value={args[argName]}
								type={formerProperty?.type}
								oneOf={formerProperty?.oneOf}
								required={formerProperty?.required}
								pattern={formerProperty?.pattern}
								valid={inputCheck[argName]}
								defaultValue={defaultValues?.[argName] ??
									structuredClone($state.snapshot(formerProperty?.default))}
								enum_={dynamicEnums?.[argName] ?? formerProperty?.enum}
								format={formerProperty?.format}
								contentEncoding={formerProperty?.contentEncoding}
								customErrorMessage={formerProperty?.customErrorMessage}
								properties={formerProperty?.properties}
								order={formerProperty?.order}
								nestedRequired={formerProperty?.required}
								itemsType={formerProperty?.items}
								disabled={disabledArgs.includes(argName) || disabled || formerProperty?.disabled}
								{compact}
								{variableEditor}
								{itemPicker}
								{pickForField}
								password={linkedSecret == argName}
								extra={formerProperty}
								{showSchemaExplorer}
								simpleTooltip={schemaFieldTooltip[argName]}
								{onlyMaskPassword}
								nullable={formerProperty?.nullable}
								title={formerProperty?.title}
								placeholder={formerProperty?.placeholder}
								orderEditable={dndConfig != undefined}
								otherArgs={args}
								{helperScript}
								{lightHeader}
								hideNested={typeof diff[argName].diff === 'object'}
								diffStatus={undefined}
								{appPath}
								{computeS3ForceViewerPolicies}
								{workspace}
								{css}
								{displayType}
							/>
						</div>
					{/if}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="flex flex-row items-center {largeGap ? 'pb-4' : ''} "
						onclick={() => {
							dispatch('click', argName)
						}}
					>
						{#if args && typeof args == 'object' && schema?.properties[argName]}
							<!-- {argName}
							{args == undefined}
							{JSON.stringify(args?.[argName])} -->
							{#if !hidden[argName]}
								<ArgInput
									on:change={() => {
										dispatch('change')
									}}
									on:nestedChange={() => {
										dispatch('nestedChange')
									}}
									on:acceptChange={(e) => dispatch('acceptChange', e.detail)}
									on:rejectChange={(e) => dispatch('rejectChange', e.detail)}
									on:keydownCmdEnter={() => dispatch('keydownCmdEnter')}
									{disablePortal}
									{resourceTypes}
									{prettifyHeader}
									autofocus={i == 0 && autofocus ? true : null}
									label={argName}
									description={schema.properties[argName].description}
									bind:value={args[argName]}
									type={schema.properties[argName].type}
									oneOf={schema.properties[argName].oneOf}
									required={schema?.required?.includes(argName)}
									pattern={schema.properties[argName].pattern}
									bind:valid={inputCheck[argName]}
									defaultValue={defaultValues?.[argName] ??
										structuredClone($state.snapshot(schema.properties[argName].default))}
									enum_={dynamicEnums?.[argName] ?? schema.properties[argName].enum}
									format={schema.properties[argName].format}
									contentEncoding={schema.properties[argName].contentEncoding}
									customErrorMessage={schema.properties[argName].customErrorMessage}
									bind:properties={schema.properties[argName].properties}
									bind:order={schema.properties[argName].order}
									nestedRequired={schema.properties[argName]?.required}
									itemsType={schema.properties[argName].items}
									disabled={disabledArgs.includes(argName) ||
										disabled ||
										schema.properties[argName].disabled}
									{compact}
									{variableEditor}
									{itemPicker}
									bind:pickForField
									password={linkedSecret == argName}
									extra={schema.properties[argName]}
									{showSchemaExplorer}
									simpleTooltip={schemaFieldTooltip[argName]}
									{onlyMaskPassword}
									nullable={schema.properties[argName].nullable}
									title={schema.properties[argName].title}
									placeholder={schema.properties[argName].placeholder}
									orderEditable={dndConfig != undefined}
									otherArgs={args}
									{helperScript}
									{lightHeader}
									diffStatus={diff[argName] ?? undefined}
									{nestedParent}
									{shouldDispatchChanges}
									{nestedClasses}
									{appPath}
									{computeS3ForceViewerPolicies}
									{workspace}
									{css}
									{displayType}
								>
									{#snippet actions()}
										{@render actions_render?.()}
										{#if linkedSecretCandidates?.includes(argName)}
											<div>
												<ToggleButtonGroup
													selected={linkedSecret == argName ? 'secret' : 'inlined'}
													on:selected={(e) => {
														if (e.detail === 'secret') {
															linkedSecret = argName
														} else if (linkedSecret == argName) {
															linkedSecret = undefined
														}
													}}
												>
													{#snippet children({ item })}
														<ToggleButton
															value="inlined"
															size="sm"
															label="Inlined"
															tooltip="The value is inlined in the resource and thus has no special treatment."
															{item}
														/>
														<ToggleButton
															position="right"
															value="secret"
															size="sm"
															label="Secret"
															tooltip="The value will be stored in a newly created linked secret variable at the same path. That variable can be permissioned differently, will be treated as a secret the UI, operators will not be able to load it and every access will generate a corresponding audit log."
															{item}
														/>
													{/snippet}
												</ToggleButtonGroup>
											</div>{/if}
									{/snippet}
								</ArgInput>
							{/if}
							<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	{:else if !shouldHideNoInputs}
		<div class="text-secondary text-sm">No inputs</div>
	{/if}
</div>
{#if !noVariablePicker}
	<ItemPicker
		bind:this={itemPicker}
		pickCallback={(path, _) => {
			if (pickForField && args) {
				args[pickForField] = '$var:' + path
			}
		}}
		itemName="Variable"
		tooltip="Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow."
		documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
		extraField="path"
		loadItems={async () =>
			(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
				name: x.path,
				...x
			}))}
	>
		{#snippet submission()}
			<div>
				<Button
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: Plus }}
					on:click={() => variableEditor?.initNew?.()}
				>
					New variable
				</Button>
			</div>
		{/snippet}
	</ItemPicker>

	<VariableEditor bind:this={variableEditor} />
{/if}
