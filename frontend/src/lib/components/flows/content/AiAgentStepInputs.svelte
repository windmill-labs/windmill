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

	/**
	 * The rows this step's form has open, for the run form, which has no add-field control of its
	 * own and would otherwise not offer a field that was added here and left at its default: to a
	 * reader of the stored transforms alone, that is indistinguishable from a field nobody touched.
	 */
	export function openAgentFields(key: string | undefined): string[] {
		return (key ? openFieldsByStep.get(key) : undefined) ?? []
	}
</script>

<script lang="ts">
	import type { Schema } from '$lib/common'
	import { deepEqual } from 'fast-equals'
	import { type InputTransform } from '$lib/gen'
	import { allTrue, type DynamicInput as DynamicInputTypes } from '$lib/utils'
	import { getContext, untrack, type Snippet } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { Button } from '$lib/components/common'
	import StepInputsGen from '$lib/components/copilot/StepInputsGen.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import InputTransformPickers from '$lib/components/InputTransformPickers.svelte'
	import { useWorkspaceStorageConfigured } from '$lib/components/inputTransformEnv.svelte'
	import type ItemPicker from '$lib/components/ItemPicker.svelte'
	import type VariableEditor from '$lib/components/VariableEditor.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ResizeTransitionWrapper from '$lib/components/common/ResizeTransitionWrapper.svelte'
	import FieldHeader from '$lib/components/FieldHeader.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { AlertTriangle, Plus, X } from 'lucide-svelte'
	import type { PickableProperties } from '../previousResults'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { toolEnabledName, type AgentTool } from '../agentToolUtils'
	import {
		AGENT_FIELDS,
		AGENT_FIELD_BY_KEY,
		AGENT_HISTORY_KEYS,
		AGENT_MEMORY_DOCS_URL,
		AGENT_TOOLS_ROW,
		AGENT_FIELD_GROUPS,
		agentFieldAppliesTo,
		agentFieldServes,
		agentOutputType,
		agentMemoryMode,
		historyInputApplies,
		type AgentMemoryMode,
		initialVisibleAgentFields,
		type AgentFieldGroup,
		type AgentFieldSpec,
		type AgentHistoryKey
	} from '../agentFormFields'
	import AgentToolRoster from './AgentToolRoster.svelte'
	import AgentMemoryNotes from './AgentMemoryNotes.svelte'
	import { memoryOptionLabel, memoryPropertyFor } from '../flowInfers'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

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
		/** A linked agent's memory and output type, once its config has loaded: whether it keeps
		 *  managed memory decides which history inputs the step offers, and its output type which of
		 *  the flow-local inputs a run reads. */
		linkedBrain?: { memory: unknown; output_type: unknown } | undefined
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
		toolPickerPortal = undefined,
		linkedBrain = undefined
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)

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

	const s3Storage = useWorkspaceStorageConfigured(() => ws)

	// The per-field copilot only ever writes a JavaScript transform, so it belongs only where one can
	// be stored. On a static-only field the write lands in a key the config drops on deploy, which
	// reports success over a visible edit, or persists an expression into a saved agent that every
	// flow linking it would then have to satisfy.
	let fieldAiEnabled = $derived(enableAi && !staticOnly && !noJavascript)

	let schemaProperties = $derived((schema?.properties ?? {}) as Record<string, any>)

	// Which memory shape the brain edited here, or the linked agent's, holds. Unknown for an
	// expression or a linked agent that has not loaded, which keeps previous messages addable.
	let memoryMode = $derived.by((): AgentMemoryMode | undefined => {
		if ('memory' in schemaProperties) {
			const transform = args?.memory
			return transform == undefined || transform.type === 'static'
				? agentMemoryMode(transform?.value)
				: undefined
		}
		return linkedBrain ? agentMemoryMode(linkedBrain.memory) : undefined
	})

	// The one-of field rewrites a value that matches none of its options, so a legacy kind the step
	// still holds is offered alongside the current ones.
	let memoryFieldSchema = $derived.by(() => {
		const property = schemaProperties.memory
		const value = args?.memory?.type === 'static' ? args.memory.value : undefined
		const withLegacy = memoryPropertyFor(property, value, chatInputEnabled)
		if (withLegacy === property) return schema
		return { ...schema, properties: { ...schemaProperties, memory: withLegacy } }
	})

	function isHistoryKey(key: string): key is AgentHistoryKey {
		return (AGENT_HISTORY_KEYS as readonly string[]).includes(key)
	}

	// Offer the agent's own tools as the choices for `enabled_tools`, rather than asking for names
	// to be typed. Written into the schema because that is where `InputTransformForm` reads a
	// field's shape from; `flowInfers` hands every step its own copy, so this stays this step's.
	// A linked step gets the resource's roster here, which is the one it narrows.
	$effect(() => {
		// By what each tool is named, not the summary alone: an MCP entry is added without one and is
		// named by its resource path, so keying on `summary` would leave a whole server with no name
		// to pick. `narrow_roster` matches that path for the same reason.
		const names = tools
			.map((tool) => toolEnabledName(tool))
			.filter((name): name is string => !!name)
		const properties = schemaProperties
		untrack(() => {
			const list = properties['enabled_tools']
			if (list && !deepEqual(list.items?.enum, names)) {
				list.items = { ...(list.items ?? { type: 'string' }), enum: names }
			}
		})
	})

	// A calling agent fills only a nested agent's user message, so it could never pass the state a
	// decision needs; the worker refuses one. Same per-step schema copy as above.
	$effect(() => {
		const property = isAgentTool ? schemaProperties['output_type'] : undefined
		untrack(() => {
			if (property?.enum?.includes('decision')) {
				property.enum = property.enum.filter((value: string) => value !== 'decision')
			}
		})
	})

	let scopedFields = $derived(
		AGENT_FIELDS.filter(
			(spec) =>
				agentFieldAppliesTo(spec, schemaProperties) && (!filter || filter.includes(spec.key))
		)
	)

	// Offered when managed memory is on or an expression the form cannot read, not while a linked
	// agent's setting is unknown.
	// Unset, the memory id the run was started with applies, so the step's own id sits behind a
	// choice and the key exists only once Custom is picked.
	let memoryIsExpression = $derived(
		'memory' in schemaProperties &&
			(args?.memory?.type === 'javascript' || args?.memory?.type === 'ai')
	)
	// Names the legacy value the way the memory field's own button does, reading it wherever the mode
	// came from, so the row and the setting it points at cannot name it differently.
	let legacyMemoryNote = $derived.by(() => {
		const onThisForm = 'memory' in schemaProperties
		const label = memoryOptionLabel(
			onThisForm
				? args?.memory?.type === 'static'
					? args.memory.value
					: undefined
				: linkedBrain?.memory
		)
		return onThisForm
			? `Ignored while memory is set to ${label}.`
			: `Ignored while the agent's memory is set to ${label}.`
	})

	let memoryIdOffered = $derived(
		(memoryMode === 'managed' || memoryIsExpression) &&
			scopedFields.some((spec) => spec.key === 'memory_id')
	)

	// A history input's row follows its key: the remembered `visible` set can outlive a key that a
	// save, an undo or the AI chat removed, and a row with no value renders no field.
	function isShown(key: string): boolean {
		if (isHistoryKey(key)) {
			return args?.[key] != undefined || (key === 'memory_id' && memoryIdOffered)
		}
		return visible.has(key)
	}

	function setCustomMemoryId(on: boolean) {
		if (!args || on === (args.memory_id != undefined)) return
		if (on) {
			args.memory_id = { type: 'static', value: '' }
		} else {
			delete args.memory_id
			delete inputCheck.memory_id
		}
	}

	let outputType = $derived.by(() => {
		if (!('output_type' in schemaProperties)) return agentOutputType(linkedBrain?.output_type)
		const transform = args?.['output_type']
		return agentOutputType(transform?.type === 'static' ? transform.value : undefined)
	})

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
		const set = initialVisibleAgentFields(args, schemaProperties, outputType)
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

	// A history input's row follows its key rather than `visible`, so the run form is told about one
	// only while the step holds the key: an inherited memory id has nothing a test run could inherit.
	$effect(() => {
		const keys = [
			...[...visible].filter((key) => !isHistoryKey(key)),
			...AGENT_HISTORY_KEYS.filter((key) => args?.[key] != undefined)
		]
		untrack(() => rememberOpenFields(visibilityKey, keys))
	})

	function rowsIn(group: AgentFieldGroup): AgentFieldSpec[] {
		return scopedFields.filter(
			(spec) => spec.group === group && isShown(spec.key) && agentFieldServes(spec, outputType)
		)
	}

	function addableIn(): AgentFieldSpec[] {
		return scopedFields.filter(
			(spec) =>
				!spec.core &&
				!spec.virtual &&
				!isShown(spec.key) &&
				agentFieldServes(spec, outputType) &&
				// Memory id's row appears on its own when it is offered, so the menu never adds it.
				spec.key !== 'memory_id' &&
				!(isHistoryKey(spec.key) && !historyInputApplies(spec.key, memoryMode))
		)
	}

	function addField(spec: AgentFieldSpec) {
		// `flowInfers` re-seeds every key on load, so adding cannot mean creating the key: it means
		// showing the row. Seeded at what a run does today, so the field opens on what it overrides,
		// except where an empty value is a choice of its own rather than the absent one (`seed`).
		if (args) {
			args[spec.key] = { type: 'static', value: structuredClone(spec.seed ?? spec.implicit) }
		}
		visible.add(spec.key)
	}

	function removeField(spec: AgentFieldSpec) {
		visible.delete(spec.key)
		if (args) {
			if (isHistoryKey(spec.key)) {
				delete args[spec.key]
			} else {
				// Back to exactly what `flowInfers` seeds, so removing a field leaves no diff behind.
				// Never `delete args[key]`: the key returns on the next load, and the CLI linter requires
				// `user_message` to be present.
				args[spec.key] = { type: 'static', value: undefined }
			}
		}
		// InputTransformSchemaForm leaks these on unmount, which would pin `isValid` false forever
		// once hiding a row is routine.
		delete inputCheck[spec.key]
	}

	// Holding `enabled_tools` and naming nothing advertises no tools at all. That is a choice the
	// field has to allow, and the one the row opens on, so it says so where it is made rather than
	// leaving it to be discovered in a run. Only a static list can be read here: an expression's
	// value exists only once the run it decides is under way.
	let noToolsEnabled = $derived.by(() => {
		const transform = args?.['enabled_tools']
		return (
			transform?.type === 'static' && Array.isArray(transform.value) && transform.value.length === 0
		)
	})

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

{#snippet unsetButton(spec: AgentFieldSpec)}
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

{#snippet memoryIdHeader()}
	<div class="flex flex-col gap-1">
		<FieldHeader
			label={AGENT_FIELD_BY_KEY.memory_id.label}
			simpleTooltip={AGENT_FIELD_BY_KEY.memory_id.tooltip}
			displayType={false}
		/>
		<ToggleButtonGroup
			selected={args?.memory_id == undefined ? 'inherited' : 'custom'}
			onSelected={(next) => setCustomMemoryId(next === 'custom')}
		>
			{#snippet children({ item })}
				<ToggleButton value="inherited" label="Inherited" {item} />
				<ToggleButton value="custom" label="Custom" {item} />
			{/snippet}
		</ToggleButtonGroup>
	</div>
{/snippet}

{#snippet transformField(
	key: string,
	label: string,
	tooltip: string | undefined,
	removable: AgentFieldSpec | undefined,
	header: Snippet | undefined = undefined,
	collapsed: boolean = false
)}
	<InputTransformForm
		{previousModuleId}
		bind:arg={args[key]}
		bind:schema={
			() => (key === 'memory' ? memoryFieldSchema : schema),
			(value) => {
				if (key !== 'memory') schema = value
			}
		}
		argName={key}
		{label}
		headerTooltip={tooltip}
		hideDescription
		subtleControls
		{header}
		indentUnderHeader={false}
		{collapsed}
		animateAppear={header != undefined}
		argExtra={schemaProperties[key] ?? {}}
		bind:inputCheck={() => inputCheck[key] ?? false, (value) => (inputCheck[key] = value)}
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
		otherArgs={Object.fromEntries(Object.entries(args ?? {}).filter(([other]) => other !== key))}
	>
		{#snippet labelExtra()}
			{#if removable}
				{@render unsetButton(removable)}
			{/if}
		{/snippet}
	</InputTransformForm>
{/snippet}

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
								{#if spec.key === AGENT_TOOLS_ROW}
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
										{#if spec.key === 'memory'}
											{@render transformField(spec.key, spec.label, spec.tooltip, spec)}
											<AgentMemoryNotes
												bind:args
												{chatInputEnabled}
												historyOnStep={scopedFields.some((f) => f.key === 'previous_messages')}
												s3StorageConfigured={s3Storage.current}
											/>
										{:else if spec.key === 'memory_id' && memoryIdOffered}
											{@render transformField(
												spec.key,
												spec.label,
												spec.tooltip,
												undefined,
												memoryIdHeader,
												args?.memory_id == undefined
											)}
											{#if args?.memory_id == undefined}
												<p class="mt-1 text-xs text-secondary">
													Uses the <code>memory_id</code> the run was started with: the conversation
													id in chat mode, or the <code>memory_id</code> query parameter otherwise.
													<a
														href={AGENT_MEMORY_DOCS_URL}
														target="_blank"
														rel="noopener noreferrer"
														class="underline">Learn more</a
													>
												</p>
											{/if}
										{:else}
											{@render transformField(spec.key, spec.label, spec.tooltip, spec)}
											{#if isHistoryKey(spec.key) && !historyInputApplies(spec.key, memoryMode)}
												<p class="mt-1 text-2xs text-hint">
													{memoryMode === 'legacy'
														? legacyMemoryNote
														: `Ignored while managed memory is ${memoryMode === 'managed' ? 'on' : 'off'}.`}
												</p>
											{/if}
											{#if spec.key === 'enabled_tools' && noToolsEnabled}
												<div
													class="mt-1 flex items-center gap-1 text-2xs text-yellow-600 dark:text-yellow-400"
												>
													<AlertTriangle size={12} />
													Nothing selected: the agent runs with no tools.
												</div>
											{/if}
										{/if}
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
