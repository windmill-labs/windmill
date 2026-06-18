<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue, computeShow, type DynamicInput } from '$lib/utils'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import { Plus, RotateCcw, Trash2 } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import Badge from './common/badge/Badge.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import {
		dragHandleZone,
		SHADOW_ITEM_MARKER_PROPERTY_NAME,
		type Options as DndOptions
	} from '@windmill-labs/svelte-dnd-action'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	import type { ComponentCustomCSS } from './apps/types'
	import ResizeTransitionWrapper from './common/ResizeTransitionWrapper.svelte'
	import { twMerge } from 'tailwind-merge'

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
		linkedSecrets?: string[]
		linkedSecretCandidates?: string[] | undefined
		noVariablePicker?: boolean
		flexWrap?: boolean
		noDelete?: boolean
		displayExtraArgs?: boolean
		prettifyHeader?: boolean
		disablePortal?: boolean
		showSchemaExplorer?: boolean
		showReset?: boolean
		onlyMaskPassword?: boolean
		dndConfig?: DndOptions | undefined
		items?: { id: string; value: string }[] | undefined
		helperScript?: DynamicInput.HelperScript
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
		lightHeaderFont?: boolean
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
		chatInputEnabled?: boolean
		actions?: import('svelte').Snippet<[{ item: { id: string; value: string } }]> | undefined
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
		linkedSecrets = $bindable([]),
		linkedSecretCandidates = undefined,
		noVariablePicker = false,
		flexWrap = false,
		noDelete = false,
		displayExtraArgs = false,
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
		lightHeaderFont = false,
		computeS3ForceViewerPolicies = undefined,
		workspace = undefined,
		chatInputEnabled = false,
		actions: actions_render = undefined
	}: Props = $props()

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

	// `displayExtraArgs` disables the automatic removal of args that are absent from
	// the schema. Instead they are surfaced as fields the user can keep or remove:
	//  - kept (the default): the value stays in `args` and renders as an editable
	//    field, so existing values are never silently dropped
	//  - deleted: the user removed it; the value moves to `deletedArgs` (out of
	//    `args`) and renders with a "Deleted" badge so the deletion can be reverted
	let deletedArgs: Record<string, any> = $state({})

	let extraArgKeys = $derived(
		displayExtraArgs
			? Array.from(
					new Set([
						...Object.keys(args ?? {}).filter((k) => !keys.includes(k)),
						...Object.keys(deletedArgs)
					])
				).filter((k) => !keys.includes(k))
			: []
	)

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key) && args) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	function inferArgType(value: any): string | undefined {
		if (value === null || value === undefined) return undefined
		if (Array.isArray(value)) return 'array'
		const t = typeof value
		if (t === 'string' || t === 'number' || t === 'boolean' || t === 'object') return t
		return undefined
	}

	function deleteExtraArg(key: string) {
		if (args && key in args) {
			deletedArgs[key] = args[key]
			delete args[key]
		}
		dispatch('change')
	}

	function revertExtraArg(key: string) {
		if (!(key in deletedArgs)) return
		if (!args) args = {}
		args[key] = deletedArgs[key]
		delete deletedArgs[key]
		dispatch('change')
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
				if (
					!deepEqual(schema.properties, n) ||
					!deepEqual(Object.keys(schema.properties), Object.keys(n))
				) {
					schema.properties = n
				}
			}
			let nkeys = Object.keys(schema.properties ?? {})

			if (!deepEqual(keys, nkeys)) {
				keys = nkeys
				dispatch('change')
			}
		}
		// let missingKeys = keys.filter((x) => args && !(x in args))
		// console.log('missingKeys', missingKeys)
		// missingKeys.forEach((x) => {
		// 	if (args) {
		// 		args[x] = undefined
		// 	}
		// })

		if (!noDelete && !displayExtraArgs && hasExtraKeys()) {
			removeExtraKey()
		}
	}

	let hidden: Record<string, boolean> = $state({})
	let fields = $derived(items ?? keys.map((x) => ({ id: x, value: x })))

	function handleHiddenFields(schema: Schema | any, args: Record<string, any>) {
		for (const x of fields) {
			const prop = schema?.properties?.[x.value]
			if (prop?.hideWhenChatEnabled && chatInputEnabled) {
				if (!hidden[x.value]) {
					hidden[x.value] = true
					delete args[x.value]
					inputCheck[x.value] = true
				}
				continue
			}
			if (prop?.showExpr) {
				if (computeShow(x.value, prop.showExpr, args)) {
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

	$effect.pre(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})

	$effect.pre(() => {
		schema?.order
		Object.keys(schema?.properties ?? {})
		schema && (untrack(() => reorder()), (hidden = {}))
	})
	// Drop surfaced-deletion state when `args` is replaced wholesale (e.g. the
	// editor instance is reused for a different runnable). In-place edits from
	// deleteExtraArg/revertExtraArg keep the same reference, so they don't reset.
	let prevArgsRef: Record<string, any> | undefined = undefined
	$effect.pre(() => {
		args
		untrack(() => {
			if (displayExtraArgs && args !== prevArgsRef) {
				prevArgsRef = args
				if (Object.keys(deletedArgs).length > 0) {
					deletedArgs = {}
				}
			}
		})
	})
	$effect.pre(() => {
		;[schema, args]

		if (args && typeof args == 'object') {
			let hasShowExpr = false
			let hasHideWhenChatEnabled = false
			for (const key of fields) {
				const prop = schema?.properties?.[key.value]
				if (prop?.showExpr) {
					hasShowExpr = true
				}
				if (prop?.hideWhenChatEnabled && chatInputEnabled) {
					hasHideWhenChatEnabled = true
				}
			}
			if (!hasShowExpr && !hasHideWhenChatEnabled) {
				return
			}
			for (const key in args) {
				args[key]
			}
		}
		untrack(() => handleHiddenFields(schema, args ?? {}))
	})
	$effect.pre(() => {
		isValid = allTrue(inputCheck ?? {})
	})
</script>

{#if showReset}
	<div class="flex flex-row-reverse w-full">
		<Button size="xs" color="light" on:click={() => setDefaults()}>
			Reset args to runnable's defaults
		</Button>
	</div>
{/if}
<!-- {JSON.stringify(schema.order)} -->
<!-- {JSON.stringify(schema)} -->
<div
	class="w-full {className} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''} {nestedClasses}"
	use:dragHandleZone={dndConfig ?? { items: [], dragDisabled: true }}
	onfinalize={bubble('finalize')}
	onconsider={bubble('consider')}
>
	{#if keys.length > 0 && args}
		{#each fields as item, i (item.id)}
			{@const argName = item.value}
			{@const prop = schema?.properties?.[argName]}
			<ResizeTransitionWrapper
				vertical
				class={twMerge(
					typeof diff[argName] === 'object' &&
						diff[argName].diff !== 'same' &&
						'bg-red-300 dark:bg-red-800 rounded-md',
					item[SHADOW_ITEM_MARKER_PROPERTY_NAME] &&
						'!visible border-2 border-dashed border-blue-300 dark:border-blue-600 bg-blue-50 dark:bg-blue-900/20 rounded-md [&>*]:invisible'
				)}
				innerClass="w-full"
			>
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				{#if !hiddenArgs.includes(argName) && keys.includes(argName)}
					{#if typeof diff[argName] === 'object' && diff[argName].oldSchema}
						{@const formerProperty = diff[argName].oldSchema}
						<div class="px-2">
							<ArgInput
								{lightHeaderFont}
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
								password={linkedSecrets.includes(argName)}
								extra={formerProperty}
								{showSchemaExplorer}
								simpleTooltip={schemaFieldTooltip[argName]}
								{onlyMaskPassword}
								nullable={formerProperty?.nullable}
								title={formerProperty?.title}
								placeholder={formerProperty?.placeholder}
								orderEditable={dndConfig != undefined}
								otherArgs={{ ...args, [argName]: undefined }}
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
						class="flex flex-row items-center {largeGap ? 'pb-4' : 'pb-2'} "
						onclick={() => {
							dispatch('click', argName)
						}}
					>
						{#if args && typeof args == 'object' && prop}
							<!-- {argName}
							{args == undefined}
							{JSON.stringify(args?.[argName])} -->
							{#if !hidden[argName]}
								<ArgInput
									{lightHeaderFont}
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
									description={prop?.description}
									bind:value={args[argName]}
									type={prop?.type}
									oneOf={prop?.oneOf}
									required={schema?.required?.includes(argName)}
									pattern={prop?.pattern}
									bind:valid={inputCheck[argName]}
									defaultValue={defaultValues?.[argName] ??
										structuredClone($state.snapshot(prop?.default))}
									enum_={dynamicEnums?.[argName] ?? prop?.enum}
									format={prop?.format}
									contentEncoding={prop?.contentEncoding}
									customErrorMessage={prop?.customErrorMessage}
									bind:properties={
										() => prop?.properties,
										(v) => {
											if (prop) prop.properties = v
										}
									}
									bind:order={
										() => prop?.order,
										(v) => {
											if (prop) prop.order = v
										}
									}
									nestedRequired={prop?.required}
									itemsType={prop?.items}
									disabled={disabledArgs.includes(argName) || disabled || prop?.disabled}
									{compact}
									{variableEditor}
									{itemPicker}
									bind:pickForField
									password={linkedSecrets.includes(argName)}
									extra={prop}
									{showSchemaExplorer}
									simpleTooltip={schemaFieldTooltip[argName]}
									{onlyMaskPassword}
									nullable={prop?.nullable}
									title={prop?.title}
									placeholder={prop?.placeholder}
									orderEditable={dndConfig != undefined}
									otherArgs={{ ...args, [argName]: undefined }}
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
										{@render actions_render?.({ item })}
										{#if linkedSecretCandidates?.includes(argName)}
											<div class="relative">
												<ToggleButtonGroup
													selected={linkedSecrets.includes(argName) ? 'secret' : 'inlined'}
													on:selected={(e) => {
														if (e.detail === 'secret') {
															if (!linkedSecrets.includes(argName)) {
																linkedSecrets = [...linkedSecrets, argName]
															}
														} else {
															linkedSecrets = linkedSecrets.filter((s) => s !== argName)
														}
													}}
												>
													{#snippet children({ item })}
														<ToggleButton
															value="inlined"
															label="Inlined"
															tooltip="The value is inlined in the resource and thus has no special treatment."
															{item}
														/>
														<ToggleButton
															value="secret"
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
			</ResizeTransitionWrapper>
		{/each}
	{:else if !shouldHideNoInputs && extraArgKeys.length == 0}
		<div class="text-secondary text-xs">No inputs</div>
	{/if}
</div>
{#if displayExtraArgs && extraArgKeys.length > 0 && args}
	<div class="w-full flex flex-col gap-2 {className} {nestedClasses}">
		{#each extraArgKeys as argName (argName)}
			{@const isDeleted = argName in deletedArgs}
			<div class="flex flex-row items-start gap-2 pb-2">
				<div class="grow min-w-0 {isDeleted ? 'opacity-60' : ''}">
					{#if isDeleted}
						<ArgInput
							{resourceTypes}
							{prettifyHeader}
							{lightHeaderFont}
							{lightHeader}
							label={argName}
							value={deletedArgs[argName]}
							type={inferArgType(deletedArgs[argName])}
							disabled
							{compact}
							{disablePortal}
							{onlyMaskPassword}
							{displayType}
						/>
					{:else}
						<ArgInput
							{resourceTypes}
							{prettifyHeader}
							{lightHeaderFont}
							{lightHeader}
							label={argName}
							bind:value={args[argName]}
							type={inferArgType(args?.[argName])}
							disabled={disabled || disabledArgs.includes(argName)}
							{compact}
							{disablePortal}
							{onlyMaskPassword}
							{displayType}
							on:change={() => dispatch('change')}
						/>
					{/if}
				</div>
				<div class="flex flex-row items-center gap-1 pt-1 shrink-0">
					{#if isDeleted}
						<Badge
							color="red"
							small
							baseClass="font-normal"
							title="This field is not present in the runnable's schema and has been removed. Revert to keep it."
						>
							Deleted
						</Badge>
					{:else}
						<Badge
							color="gray"
							small
							baseClass="font-normal"
							title="This field is not present in the runnable's schema. It is ignored at runtime."
						>
							Not in schema
						</Badge>
					{/if}
					{#if !disabled}
						{#if isDeleted}
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: RotateCcw }}
								on:click={() => revertExtraArg(argName)}
							>
								Revert
							</Button>
						{:else}
							<Button
								variant="subtle"
								unifiedSize="xs"
								iconOnly
								startIcon={{ icon: Trash2 }}
								title="Delete this field"
								on:click={() => deleteExtraArg(argName)}
							/>
						{/if}
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}
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
					variant="default"
					unifiedSize="md"
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
