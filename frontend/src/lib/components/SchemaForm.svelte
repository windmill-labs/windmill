<script lang="ts">
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
	import { createEventDispatcher } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { dragHandleZone, type Options as DndOptions } from '@windmill-labs/svelte-dnd-action'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils'
	import type { ComponentCustomCSS } from './apps/types'

	export let schema: Schema | any
	export let hiddenArgs: string[] = []
	export let schemaFieldTooltip: Record<string, string> = {}
	export let args: Record<string, any> = {}
	export let disabledArgs: string[] = []
	export let disabled = false

	export let isValid: boolean = true
	export let autofocus = false
	export let defaultValues: Record<string, any> = {}

	export let shouldHideNoInputs: boolean = false
	export let compact = false
	export let linkedSecret: string | undefined = undefined
	export let linkedSecretCandidates: string[] | undefined = undefined
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false
	export let prettifyHeader = false
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let showReset = false
	export let onlyMaskPassword = false
	export let dndConfig: DndOptions | undefined = undefined
	export let items: { id: string; value: string }[] | undefined = undefined
	export let helperScript:
		| { type: 'inline'; path?: string; lang: Script['language']; code: string }
		| { type: 'hash'; hash: string }
		| undefined = undefined
	export let lightHeader = false
	export let diff: Record<string, SchemaDiff> = {}
	export let nestedParent: { label: string; nestedParent: any | undefined } | undefined = undefined
	export let shouldDispatchChanges = false
	export let nestedClasses = ''
	export let dynamicEnums: Record<string, any> = {}
	export let largeGap: boolean = false
	export let css: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined
	export let displayType: boolean = true

	export let appPath: string | undefined = undefined

	export let computeS3ForceViewerPolicies:
		| (() =>
				| {
						allowed_resources: string[]
						allow_user_resources: boolean
						allow_workspace_resource: boolean
						file_key_regex: string
				  }
				| undefined)
		| undefined = undefined
	export let workspace: string | undefined = undefined

	const dispatch = createEventDispatcher()

	let inputCheck: { [id: string]: boolean } = {}

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	export function setDefaults() {
		const nargs = structuredClone(defaultValues)

		Object.keys(schema?.properties ?? {}).forEach((key) => {
			if (schema?.properties[key].default != undefined && args[key] == undefined) {
				let value = schema?.properties[key].default
				nargs[key] = value === 'object' ? structuredClone(value) : value
			}
		})
		args = nargs
	}

	let keys: string[]
	$: keys = Array.isArray(schema?.order) ? schema?.order : Object.keys(schema?.properties ?? {})

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	let pickForField: string | undefined
	let itemPicker: ItemPicker | undefined = undefined
	let variableEditor: VariableEditor | undefined = undefined

	let resourceTypes: string[] | undefined = undefined

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	loadResourceTypes()

	$: schema && (reorder(), (hidden = {}))

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

	$: fields = items ?? keys.map((x) => ({ id: x, value: x }))

	let hidden: Record<string, boolean> = {}

	function handleHiddenFields(schema: Schema | any, args: Record<string, any>) {
		for (const x of fields) {
			if (schema?.properties[x.value]?.showExpr) {
				if (computeShow(x.value, schema.properties[x.value].showExpr, args)) {
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

	$: handleHiddenFields(schema, args)

	$: isValid = allTrue(inputCheck ?? {})
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
	class="w-full {$$props.class} {flexWrap
		? 'flex flex-row flex-wrap gap-x-6 '
		: ''} {nestedClasses}"
	use:dragHandleZone={dndConfig ?? { items: [], dragDisabled: true }}
	on:finalize
	on:consider
>
	{#if keys.length > 0}
		{#each fields as item, i (item.id)}
			{@const argName = item.value}
			<div
				class={typeof diff[argName] === 'object' && diff[argName].diff !== 'same'
					? 'bg-red-300 dark:bg-red-800 rounded-md'
					: ''}
			>
				<!-- svelte-ignore a11y-click-events-have-key-events -->
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
								defaultValue={defaultValues?.[argName] ?? structuredClone(formerProperty?.default)}
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
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						class="flex flex-row items-center {largeGap ? 'pb-4' : ''} "
						on:click={() => {
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
										structuredClone(schema.properties[argName].default)}
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
									<svelte:fragment slot="actions">
										<slot name="actions" />
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
													let:item
												>
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
												</ToggleButtonGroup>
											</div>{/if}</svelte:fragment
									>
								</ArgInput>
							{/if}
							<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
							<!-- svelte-ignore a11y-no-static-element-interactions -->
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
			if (pickForField) {
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
		<div slot="submission">
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
	</ItemPicker>

	<VariableEditor bind:this={variableEditor} />
{/if}
