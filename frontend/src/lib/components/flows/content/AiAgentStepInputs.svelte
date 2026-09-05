<script module lang="ts">
	// A field added but not yet filled looks exactly like an unset one, so leaving the step and
	// coming back would drop the row. Remembering which fields are open per step keeps it. Bounded,
	// or a long-lived tab keeps one entry for every agent step it ever opened.
	const openFieldsByStep = new Map<string, string[]>()
	const MAX_REMEMBERED_STEPS = 50

	function rememberOpenFields(key: string | undefined, keys: string[]) {
		if (!key) return
		openFieldsByStep.delete(key)
		openFieldsByStep.set(key, keys)
		while (openFieldsByStep.size > MAX_REMEMBERED_STEPS) {
			const oldest = openFieldsByStep.keys().next().value
			if (oldest === undefined) break
			openFieldsByStep.delete(oldest)
		}
	}
</script>

<script lang="ts">
	import type { Schema } from '$lib/common'
	import { type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue, type DynamicInput as DynamicInputTypes } from '$lib/utils'
	import { getContext, untrack } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { Button } from '$lib/components/common'
	import StepInputsGen from '$lib/components/copilot/StepInputsGen.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import InputTransformPickers from '$lib/components/InputTransformPickers.svelte'
	import { useS3StorageConfigured } from '$lib/components/inputTransformEnv.svelte'
	import type ItemPicker from '$lib/components/ItemPicker.svelte'
	import type VariableEditor from '$lib/components/VariableEditor.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ResizeTransitionWrapper from '$lib/components/common/ResizeTransitionWrapper.svelte'
	import { Plus, X } from 'lucide-svelte'
	import type { PickableProperties } from '../previousResults'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import type { AgentTool } from '../agentToolUtils'
	import {
		AGENT_FIELDS,
		AGENT_FIELD_GROUPS,
		agentFieldAppliesTo,
		initialVisibleAgentFields,
		type AgentFieldGroup,
		type AgentFieldSpec
	} from '../agentFormFields'
	import AgentToolRoster from './AgentToolRoster.svelte'

	interface Props {
		schema: Schema | { properties?: Record<string, any> }
		/** Required: every row below indexes it directly, and a first render against an unset one
		 *  would throw before the effect that normalises it could run. */
		args: Record<string, InputTransform | any>
		isValid?: boolean
		extraLib?: string
		previousModuleId?: string | undefined
		/** Restrict the form to these keys, for a surface that edits only part of an agent. */
		filter?: string[] | undefined
		pickableProperties?: PickableProperties | undefined
		enableAi?: boolean
		class?: string
		helperScript?: DynamicInputTypes.HelperScript
		isAgentTool?: boolean
		allowedAiTransforms?: string[] | undefined
		chatInputEnabled?: boolean
		workspace?: string | undefined
		/** Identifies the step, so the fields it has open survive leaving and coming back. */
		visibilityKey?: string
		tools?: AgentTool[]
		/** Offer only static values, for a surface whose store cannot hold anything else — a saved
		 *  agent's config is plain JSON, so an expression on a brain field would be dropped on save.
		 *  A step's own agent has no such limit: its transforms are evaluated per run. */
		staticOnly?: boolean
		/** Show the agent without letting anything about it be changed, for a viewer who has read
		 *  access only. The fields are made inert rather than merely losing their buttons: a field
		 *  left editable would look saved while every write behind it is rejected. */
		readOnly?: boolean
		/** Passed to every field, for a nested agent used as a tool: its brain has to stand on its
		 *  own as `staticOnly` does, but its `user_message` is still the agent's to fill. */
		noConnect?: boolean
		noJavascript?: boolean
		onSelectTool?: (toolId: string) => void
		/** Adds a tool to this agent. Without it the roster is read-only, as it is for a linked
		 *  agent, whose tools belong to the agent rather than to the step. */
		onAddTool?: (detail: { kind: string; script?: any; flow?: any; inlineScript?: any }) => void
		/** Removes a tool from this agent. Same rule as `onAddTool`: absent where the tools are not
		 *  the step's to change. */
		onDeleteTool?: (toolId: string) => void
		/** Where the tool picker's popover belongs, for a surface that is not the flow editor. */
		toolPickerPortal?: string
	}

	let {
		schema = $bindable(),
		args = $bindable(),
		isValid = $bindable(),
		extraLib = $bindable(),
		previousModuleId = undefined,
		filter = undefined,
		pickableProperties = undefined,
		enableAi = false,
		class: clazz = '',
		helperScript = undefined,
		isAgentTool = false,
		// Reproduces InputTransformSchemaForm's own default rather than the call site's value:
		// `undefined` lets any field become an AI transform, and InputTransformForm rewrites
		// `arg.type` to 'ai' on mount for every static-undefined field when it does.
		allowedAiTransforms = isAgentTool ? undefined : [],
		chatInputEnabled = false,
		workspace,
		visibilityKey = undefined,
		tools = [],
		staticOnly = false,
		readOnly = false,
		noConnect = false,
		noJavascript = false,
		onSelectTool = undefined,
		onAddTool = undefined,
		onDeleteTool = undefined,
		toolPickerPortal = undefined
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let inputCheck: { [id: string]: boolean } = $state({})

	$effect(() => {
		isValid = allTrue(inputCheck) ?? false
	})

	$effect(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})

	export function setArgs(nargs: Record<string, InputTransform | any>) {
		args = nargs
	}

	let pickForField: string | undefined = $state()
	let itemPicker: ItemPicker | undefined = $state(undefined)
	let variableEditor: VariableEditor | undefined = $state(undefined)

	const s3Storage = useS3StorageConfigured(() => ws)

	// The per-field copilot only ever writes a JavaScript transform, so it belongs only where one can
	// be stored. On a static-only field the write lands in a key the config drops on deploy, which
	// reports success over a visible edit, or persists an expression into a saved agent that every
	// flow linking it would then have to satisfy.
	let fieldAiEnabled = $derived(enableAi && !staticOnly && !noJavascript)

	let schemaProperties = $derived((schema?.properties ?? {}) as Record<string, any>)

	let scopedFields = $derived(
		AGENT_FIELDS.filter(
			(spec) =>
				agentFieldAppliesTo(spec, schemaProperties) && (!filter || filter.includes(spec.key))
		)
	)

	let outputType = $derived.by(() => {
		const transform = args?.['output_type']
		return transform && transform.type === 'static' ? transform.value : undefined
	})
	let imageOutput = $derived(outputType === 'image')

	// Which fields have a row. Never derived from `args`, or emptying a textbox would make its row
	// vanish under the cursor: this only ever grows, and the x is the one thing that shrinks it.
	// Read once, on purpose: the set is seeded here and grows from there, so re-reading the key would
	// be a rebuild that throws away everything the user has opened.
	let visible = $state(
		new SvelteSet<string>(openFieldsByStep.get(untrack(() => visibilityKey) ?? '') ?? [])
	)

	// A field set from anywhere else — an undo, a schema that arrived late — brings its row back on
	// its own.
	$effect(() => {
		const set = initialVisibleAgentFields(args, schemaProperties)
		untrack(() => {
			for (const key of set) visible.add(key)
		})
	})

	// The copilot hands its expressions to whichever row is mounted for the argument, so a field it
	// targets needs one before it can take the value. Watched here rather than left to the union
	// above, which only sees a field once something has already written to it.
	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}
	let pendingExprKeys = $derived(
		exprsToSet ? Object.keys($exprsToSet ?? {}).filter((key) => $exprsToSet?.[key]) : []
	)
	$effect(() => {
		const keys = pendingExprKeys
		untrack(() => {
			for (const key of keys) {
				if (key in schemaProperties) visible.add(key)
			}
		})
	})

	$effect(() => {
		const keys = [...visible]
		untrack(() => rememberOpenFields(visibilityKey, keys))
	})

	function rowsIn(group: AgentFieldGroup): AgentFieldSpec[] {
		return scopedFields.filter(
			(spec) => spec.group === group && visible.has(spec.key) && !(imageOutput && spec.textOnly)
		)
	}

	function addableIn(): AgentFieldSpec[] {
		return scopedFields.filter(
			(spec) =>
				!spec.core && !spec.virtual && !visible.has(spec.key) && !(imageOutput && spec.textOnly)
		)
	}

	function addField(spec: AgentFieldSpec) {
		// `flowInfers` re-seeds every key on load, so adding cannot mean creating the key: it means
		// showing the row, seeded at what a run does today so the field opens on what it overrides.
		if (args) {
			args[spec.key] = { type: 'static', value: structuredClone(spec.implicit) }
		}
		visible.add(spec.key)
	}

	function removeField(spec: AgentFieldSpec) {
		visible.delete(spec.key)
		if (args) {
			// Back to exactly what `flowInfers` seeds, so removing a field leaves no diff behind.
			// Never `delete args[key]`: the key returns on the next load, and the CLI linter requires
			// `user_message` to be present.
			args[spec.key] = { type: 'static', value: undefined }
		}
		// InputTransformSchemaForm leaks these on unmount, which would pin `isValid` false forever
		// once hiding a row is routine.
		delete inputCheck[spec.key]
	}

	let emptyArgNames = $derived(
		[...visible].filter((key) => {
			if (!(key in schemaProperties)) return false
			const transform = args?.[key]
			if (!transform) return false
			return (
				(transform.type === 'static' && !transform.value) ||
				(transform.type === 'javascript' && !transform.expr)
			)
		})
	)
</script>

{#snippet addFieldMenu()}
	{@const candidates = addableIn()}
	{#if candidates.length > 0}
		<DropdownV2 placement="bottom-start" customMenu class="justify-start">
			{#snippet buttonReplacement()}
				<Button variant="default" unifiedSize="md" startIcon={{ icon: Plus }}>Add a field</Button>
			{/snippet}
			{#snippet menu({ close })}
				<!-- Laid out as SelectDropdown lays out a resource list, so the two menus in this form
				     behave the same: full-width rows, and a bordered heading per group. -->
				<div
					class="flex flex-col w-80 rounded-md bg-surface-input drop-shadow-base overflow-y-auto max-h-[50vh]"
				>
					{#each AGENT_FIELD_GROUPS as menuGroup (menuGroup.id)}
						{@const groupCandidates = candidates.filter((spec) => spec.group === menuGroup.id)}
						{#if groupCandidates.length > 0}
							<div class="px-4 pt-3 pb-1 text-2xs font-normal uppercase text-secondary">
								{menuGroup.label}
							</div>
							{#each groupCandidates as spec (spec.key)}
								<Button
									variant="subtle"
									unifiedSize="md"
									onClick={() => {
										addField(spec)
										close()
									}}
									wrapperClasses="w-full"
									btnClasses="w-full !h-auto !justify-start !text-left !py-2 !px-4 !font-normal !text-xs text-primary !rounded-none"
								>
									<div class="w-full">
										{spec.label}
										{#if spec.defaultHint}
											<div class="text-2xs text-secondary">{spec.defaultHint}</div>
										{/if}
									</div>
								</Button>
							{/each}
						{/if}
					{/each}
				</div>
			{/snippet}
		</DropdownV2>
	{/if}
{/snippet}

<div class="w-full mb-6 {clazz}">
	<!-- Not offered on a static-only surface, where what it fills a field with is a JavaScript
	     expression such a store cannot hold; nor on a tool, whose empty fields are the ones the
	     agent above fills at run time. -->
	{#if enableAi && !staticOnly && !isAgentTool && !readOnly}
		<div class="pt-2">
			<StepInputsGen {pickableProperties} argNames={emptyArgNames} {schema} />
		</div>
	{/if}

	<div class="flex flex-col gap-8 pt-4">
		{#each AGENT_FIELD_GROUPS as group (group.id)}
			{@const rows = rowsIn(group.id)}
			{#if rows.length > 0}
				<div class="w-full flex flex-col">
					<h2 class="mb-1 text-2xs font-normal uppercase text-secondary">{group.label}</h2>
					<div class="flex flex-col gap-6">
						{#each rows as spec (spec.key)}
							<ResizeTransitionWrapper innerClass="w-full" vertical>
								{#if spec.virtual}
									<AgentToolRoster
										{tools}
										{onSelectTool}
										{onAddTool}
										{onDeleteTool}
										pickerPortal={toolPickerPortal}
									/>
								{:else}
									<!-- Inert rather than merely button-less: every control below writes into
									     `args`, and a read-only viewer's edit is rejected by the server. Dimmed
									     with it, so a field that ignores a click looks like it meant to. -->
									<div class="w-full {readOnly ? 'opacity-60' : ''}" inert={readOnly}>
										<InputTransformForm
											{previousModuleId}
											bind:arg={args[spec.key]}
											bind:schema
											argName={spec.key}
											label={spec.label}
											headerTooltip={spec.tooltip}
											hideDescription
											subtleControls
											argExtra={schemaProperties[spec.key] ?? {}}
											bind:inputCheck={
												() => inputCheck[spec.key] ?? false,
												(value) => (inputCheck[spec.key] = value)
											}
											bind:extraLib={() => extraLib ?? 'missing extraLib', (v) => (extraLib = v)}
											{variableEditor}
											{itemPicker}
											bind:pickForField
											{pickableProperties}
											enableAi={fieldAiEnabled}
											{helperScript}
											{isAgentTool}
											{allowedAiTransforms}
											noDynamicToggle={staticOnly}
											noConnect={staticOnly || noConnect}
											noJavascript={staticOnly || noJavascript}
											s3StorageConfigured={s3Storage.current}
											{chatInputEnabled}
											{workspace}
											otherArgs={Object.fromEntries(
												Object.entries(args ?? {}).filter(([key]) => key !== spec.key)
											)}
										>
											{#snippet labelExtra()}
												{#if !spec.core && !readOnly}
													<Button
														variant="subtle"
														unifiedSize="2xs"
														iconOnly
														startIcon={{ icon: X }}
														wrapperClasses="ml-1"
														title="Unset {spec.label}"
														on:click={() => removeField(spec)}
													/>
												{/if}
											{/snippet}
										</InputTransformForm>
									</div>
								{/if}
							</ResizeTransitionWrapper>
						{/each}
					</div>
				</div>
			{/if}
		{/each}
		{#if !readOnly}
			{@render addFieldMenu()}
		{/if}
	</div>
</div>

<InputTransformPickers {args} {pickForField} {workspace} bind:itemPicker bind:variableEditor />
