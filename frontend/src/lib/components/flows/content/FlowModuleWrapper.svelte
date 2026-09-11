<script lang="ts">
	import { moduleSlot, savedModuleById } from '../moduleSlot'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import { type FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

	import { stepSettingDefaults } from '$lib/components/flows/flowStepSettings'
	import { emptyString } from '$lib/utils'
	import type { FlowEditorContext } from '../types'
	import FlowLoop from './FlowLoop.svelte'
	import FlowModuleComponent from './FlowModuleComponent.svelte'
	import FlowBranchAllWrapper from './FlowBranchAllWrapper.svelte'
	import FlowBranchOneWrapper from './FlowBranchOneWrapper.svelte'
	import {
		createInlineScriptModule,
		pickFlow,
		pickScript
	} from '$lib/components/flows/flowStateUtils.svelte'
	import FlowInputs from './FlowInputs.svelte'
	import { Alert } from '$lib/components/common'
	import FlowInputsFlow from './FlowInputsFlow.svelte'
	import FlowBranchesAllWrapper from './FlowBranchesAllWrapper.svelte'
	import FlowBranchesOneWrapper from './FlowBranchesOneWrapper.svelte'
	import FlowWhileLoop from './FlowWhileLoop.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { formatCron } from '$lib/utils'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	const { selectionManager, flowStateStore, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')
	const selectedId = $derived(selectionManager.getSelectedId())

	const { triggersState, triggersCount } = getContext<TriggerContext>('TriggerContext')

	let scriptKind: 'script' | 'trigger' | 'approval' | 'preprocessor' = $state('script')
	let scriptTemplate: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' = $state('script')

	// These pointers are used to easily access previewArgs of parent module, and previous module

	interface Props {
		flowModule: FlowModule
		noEditor?: boolean
		enableAi?: boolean
		savedModule?: FlowModule | undefined
		// Pointer to parent module, only defined within Branches or Loops.
		parentModule?: FlowModule | undefined
		// Pointer to previous module, for easy access to testing results
		previousModule?: FlowModule | undefined
		forceTestTab?: Record<string, boolean>
		highlightArg?: Record<string, string | undefined>
		isAgentTool?: boolean
		/** Lets an agent step add a tool through the same path as the graph's `+ Tool`. */
		flowModuleSchemaMap?: import('../map/FlowModuleSchemaMap.svelte').default
	}

	let {
		flowModule = $bindable(),
		noEditor = false,
		enableAi = false,
		savedModule = undefined,
		parentModule = $bindable(),
		previousModule = undefined,
		forceTestTab,
		highlightArg,
		isAgentTool = false,
		flowModuleSchemaMap = undefined
	}: Props = $props()

	function initializePrimaryScheduleForTriggerScript(module: FlowModule) {
		const primaryIndex = triggersState.triggers.findIndex((t) => t.isPrimary)
		if (primaryIndex === -1) {
			const primaryCfg = {
				summary: 'Scheduled poll of flow',
				args: {},
				schedule: formatCron('0 */15 * * *'),
				timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
				enabled: true,
				is_flow: true
			}
			triggersState.addDraftTrigger(triggersCount, 'schedule', undefined, primaryCfg)
		} else if (triggersState.triggers[primaryIndex].draftConfig) {
			//If there is a primary schedule draft update it
			const newCfg = { ...triggersState.triggers[primaryIndex].draftConfig }
			let updated = false
			if (!newCfg.schedule) {
				newCfg.schedule = formatCron('0 */15 * * *')
				updated = true
			}
			if (!newCfg.enabled) {
				newCfg.enabled = true
				updated = true
			}
			if (updated) {
				triggersState.triggers[primaryIndex].draftConfig = newCfg
			}
		}

		module.stop_after_if = stepSettingDefaults('early-stop', 'trigger')
	}
	async function createModuleFromScript(
		path: string,
		summary: string,
		kind: string,
		hash: string | undefined
	) {
		const [module, state] = await pickScript(
			path,
			summary,
			flowModule.id,
			hash,
			undefined,
			opWorkspace?.()
		)

		if (kind == 'approval') {
			module.suspend = stepSettingDefaults('suspend')
		}

		if (kind == 'trigger') {
			initializePrimaryScheduleForTriggerScript(module)
		}

		flowModule = module
		flowStateStore.val[module.id] = state
	}
</script>

{#if flowModule.id === selectedId}
	{#if flowModule.value.type === 'forloopflow'}
		<FlowLoop {noEditor} bind:mod={flowModule} {parentModule} {previousModule} {enableAi} />
	{:else if flowModule.value.type === 'whileloopflow'}
		<FlowWhileLoop {noEditor} bind:mod={flowModule} {previousModule} {parentModule} />
	{:else if flowModule.value.type === 'branchone'}
		<FlowBranchesOneWrapper {noEditor} {previousModule} {parentModule} {enableAi} bind:flowModule />
	{:else if flowModule.value.type === 'branchall'}
		<FlowBranchesAllWrapper {noEditor} {previousModule} {parentModule} bind:flowModule />
	{:else if flowModule.value.type === 'identity'}
		{#if selectedId == 'failure'}
			<div class="p-4">
				<Alert type="info" title="Error handlers are triggered upon non recovered errors">
					If defined, the error handler will take the error as input.
				</Alert>
			</div>
		{:else if selectedId == 'preprocessor'}
			<div class="p-4">
				<Alert
					type="info"
					title="Preprocessor is called when the flow is triggered by API or email"
				>
					It prepares arguments for the flow. Besides request arguments, the preprocessor receives a
					`wm_trigger` argument with trigger details.
				</Alert>
			</div>
		{/if}

		{#if flowModule.value.flow}
			<FlowInputsFlow
				on:pick={async ({ detail }) => {
					const { path, summary } = detail
					const [module, state] = await pickFlow(path, summary, flowModule.id, opWorkspace?.())

					flowModule = module
					flowStateStore.val[module.id] = state
				}}
			/>
		{:else}
			<FlowInputs
				{noEditor}
				summary={flowModule.summary}
				shouldDisableTriggerScripts={parentModule !== undefined ||
					previousModule !== undefined ||
					selectedId == 'failure' ||
					selectedId == 'preprocessor'}
				on:pick={async ({ detail }) => {
					const { path, summary, kind, hash } = detail
					// The picked script's summary is a default: anything already typed on the step
					// was the user's choice and outranks it.
					createModuleFromScript(
						path,
						emptyString(flowModule.summary) ? summary : flowModule.summary,
						kind,
						hash
					)
				}}
				on:new={async ({ detail }) => {
					const { language, kind, subkind, summary } = detail

					const [module, state] = await createInlineScriptModule(
						language,
						kind,
						subkind,
						flowModule.id,
						summary
					)
					scriptKind = kind
					scriptTemplate = subkind

					if (kind == 'trigger') {
						initializePrimaryScheduleForTriggerScript(module)
					}

					if (kind == 'approval') {
						module.suspend = stepSettingDefaults('suspend')
					}

					flowModule = module
					flowStateStore.val[module.id] = state
				}}
				failureModule={selectedId === 'failure'}
				preprocessorModule={selectedId === 'preprocessor'}
			/>
		{/if}
	{:else if flowModule.value.type === 'rawscript' || flowModule.value.type === 'script' || flowModule.value.type === 'flow' || flowModule.value.type === 'aiagent'}
		<FlowModuleComponent
			{noEditor}
			bind:flowModule
			{parentModule}
			{previousModule}
			failureModule={selectedId === 'failure'}
			preprocessorModule={selectedId === 'preprocessor'}
			{scriptKind}
			{scriptTemplate}
			{enableAi}
			{savedModule}
			forceTestTab={forceTestTab?.[flowModule.id]}
			highlightArg={highlightArg?.[flowModule.id]}
			{isAgentTool}
			{flowModuleSchemaMap}
		/>
	{/if}
{:else if flowModule.value.type === 'forloopflow' || flowModule.value.type == 'whileloopflow'}
	{#each flowModule.value.modules as child, index (child.id ?? index)}
		{@const slot = moduleSlot(
			() => (flowModule.value as { modules: FlowModule[] }).modules,
			child.id,
			child
		)}
		<FlowModuleWrapper
			{flowModuleSchemaMap}
			{noEditor}
			bind:flowModule={slot.get, slot.set}
			bind:parentModule={flowModule}
			previousModule={flowModule.value.modules[index - 1]}
			savedModule={savedModuleById(savedModule, child.id)}
			{enableAi}
			{forceTestTab}
			{highlightArg}
		/>
	{/each}
{:else if flowModule.value.type === 'branchone'}
	{#if selectedId === `${flowModule?.id}-branch-default`}
		<div class="h-full flex flex-col">
			<FlowCard {noEditor} title="Default branch">
				<div class="p-4">
					<p class="text-xs text-tertiary">
						Nothing to configure — this branch runs when none of the predicates match.
					</p>
				</div>
			</FlowCard>
		</div>
	{:else}
		{#each flowModule.value.default as child, index (child.id ?? index)}
			{@const slot = moduleSlot(
				() => (flowModule.value as { default: FlowModule[] }).default,
				child.id,
				child
			)}
			<FlowModuleWrapper
				{flowModuleSchemaMap}
				{noEditor}
				bind:flowModule={slot.get, slot.set}
				bind:parentModule={flowModule}
				previousModule={flowModule.value.default[index - 1]}
				savedModule={savedModuleById(savedModule, child.id)}
				{enableAi}
				{forceTestTab}
				{highlightArg}
			/>
		{/each}
	{/if}
	{#each flowModule.value.branches as branch, branchIndex (branch)}
		{#if selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchOneWrapper
				{noEditor}
				{branch}
				parentModule={flowModule}
				{previousModule}
				{enableAi}
			/>
		{:else}
			{#each branch.modules as child, index (child.id ?? index)}
				{@const slot = moduleSlot(() => branch.modules, child.id, child)}
				<FlowModuleWrapper
					{flowModuleSchemaMap}
					{noEditor}
					bind:flowModule={slot.get, slot.set}
					bind:parentModule={flowModule}
					previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					savedModule={savedModuleById(savedModule, child.id)}
					{enableAi}
					{forceTestTab}
					{highlightArg}
				/>
			{/each}
		{/if}
	{/each}
{:else if flowModule.value.type === 'branchall'}
	{#each flowModule.value.branches as branch, branchIndex (branch)}
		{#if selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchAllWrapper {noEditor} {branch} />
		{:else}
			{#each branch.modules as child, index (child.id ?? index)}
				{@const slot = moduleSlot(() => branch.modules, child.id, child)}
				<FlowModuleWrapper
					{flowModuleSchemaMap}
					{noEditor}
					bind:flowModule={slot.get, slot.set}
					bind:parentModule={flowModule}
					previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					{enableAi}
					savedModule={savedModuleById(savedModule, child.id)}
					{forceTestTab}
					{highlightArg}
				/>
			{/each}
		{/if}
	{/each}
{:else if flowModule.value.type === 'aiagent'}
	{#each flowModule.value.tools ?? [] as tool, toolIndex (toolIndex)}
		{#if selectedId === tool.id}
			<AgentToolWrapper
				{noEditor}
				bind:tool={flowModule.value.tools![toolIndex]}
				parentModule={flowModule}
				{previousModule}
				{enableAi}
				{forceTestTab}
				{highlightArg}
				siblingToolNames={flowModule.value.tools!.map((t) => t.summary ?? '')}
			/>
		{/if}
	{/each}
{/if}
