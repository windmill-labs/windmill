<script lang="ts">
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import { type FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

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
	import type { TriggerContext } from '$lib/components/triggers'
	import { formatCron } from '$lib/utils'

	const { selectedId, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const { triggersState, triggersCount } = getContext<TriggerContext>('TriggerContext')

	let scriptKind: 'script' | 'trigger' | 'approval' = $state('script')
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
	}

	let {
		flowModule = $bindable(),
		noEditor = false,
		enableAi = false,
		savedModule = undefined,
		parentModule = $bindable(),
		previousModule = undefined
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

		module.stop_after_if = {
			expr: 'result == undefined || Array.isArray(result) && result.length == 0',
			skip_if_stopped: true
		}
	}
	async function createModuleFromScript(
		path: string,
		summary: string,
		kind: string,
		hash: string | undefined
	) {
		const [module, state] = await pickScript(path, summary, flowModule.id, hash)

		if (kind == 'approval') {
			module.suspend = { required_events: 1, timeout: 1800 }
		}

		if (kind == 'trigger') {
			initializePrimaryScheduleForTriggerScript(module)
		}

		flowModule = module
		$flowStateStore[module.id] = state
	}
</script>

{#if flowModule.id === $selectedId}
	{#if flowModule.value.type === 'forloopflow'}
		<FlowLoop {noEditor} bind:mod={flowModule} {parentModule} {previousModule} {enableAi} />
	{:else if flowModule.value.type === 'whileloopflow'}
		<FlowWhileLoop {noEditor} bind:mod={flowModule} {previousModule} {parentModule} />
	{:else if flowModule.value.type === 'branchone'}
		<FlowBranchesOneWrapper {noEditor} {previousModule} {parentModule} bind:flowModule {enableAi} />
	{:else if flowModule.value.type === 'branchall'}
		<FlowBranchesAllWrapper {noEditor} {previousModule} {parentModule} bind:flowModule />
	{:else if flowModule.value.type === 'identity'}
		{#if $selectedId == 'failure'}
			<div class="p-4">
				<Alert type="info" title="Error handlers are triggered upon non recovered errors">
					If defined, the error handler will take the error as input.
				</Alert>
			</div>
		{:else if $selectedId == 'preprocessor'}
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
					const [module, state] = await pickFlow(path, summary, flowModule.id)

					flowModule = module
					$flowStateStore[module.id] = state
				}}
			/>
		{:else}
			<FlowInputs
				{noEditor}
				summary={flowModule.summary}
				shouldDisableTriggerScripts={parentModule !== undefined ||
					previousModule !== undefined ||
					$selectedId == 'failure' ||
					$selectedId == 'preprocessor'}
				on:pick={async ({ detail }) => {
					const { path, summary, kind, hash } = detail
					createModuleFromScript(path, summary, kind, hash)
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
						module.suspend = { required_events: 1, timeout: 1800 }
					}

					flowModule = module
					$flowStateStore[module.id] = state
				}}
				failureModule={$selectedId === 'failure'}
				preprocessorModule={$selectedId === 'preprocessor'}
			/>
		{/if}
	{:else if flowModule.value.type === 'rawscript' || flowModule.value.type === 'script' || flowModule.value.type === 'flow'}
		<FlowModuleComponent
			{noEditor}
			bind:flowModule
			{parentModule}
			{previousModule}
			failureModule={$selectedId === 'failure'}
			preprocessorModule={$selectedId === 'preprocessor'}
			{scriptKind}
			{scriptTemplate}
			{enableAi}
			{savedModule}
		/>
	{/if}
{:else if flowModule.value.type === 'forloopflow' || flowModule.value.type == 'whileloopflow'}
	{#each flowModule.value.modules as _, index (index)}
		<FlowModuleWrapper
			{noEditor}
			bind:flowModule={flowModule.value.modules[index]}
			bind:parentModule={flowModule}
			previousModule={flowModule.value.modules[index - 1]}
			savedModule={savedModule?.value.type === 'forloopflow' ||
			savedModule?.value.type === 'whileloopflow'
				? savedModule.value.modules[index]
				: undefined}
			{enableAi}
		/>
	{/each}
{:else if flowModule.value.type === 'branchone'}
	{#if $selectedId === `${flowModule?.id}-branch-default`}
		<div class="p-2">
			<h3 class="mb-4">Default branch</h3>
			Nothing to configure, this is the default branch if none of the predicates are met.
		</div>
	{:else}
		{#each flowModule.value.default as _, index}
			<FlowModuleWrapper
				{noEditor}
				bind:flowModule={flowModule.value.default[index]}
				bind:parentModule={flowModule}
				previousModule={flowModule.value.default[index - 1]}
				savedModule={savedModule?.value.type === 'branchone'
					? savedModule.value.default[index]
					: undefined}
				{enableAi}
			/>
		{/each}
	{/if}
	{#each flowModule.value.branches as branch, branchIndex (branchIndex)}
		{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchOneWrapper
				{noEditor}
				bind:branch={flowModule.value.branches[branchIndex]}
				parentModule={flowModule}
				{previousModule}
				{enableAi}
			/>
		{:else}
			{#each branch.modules as _, index}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule={flowModule.value.branches[branchIndex].modules[index]}
					bind:parentModule={flowModule}
					previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					savedModule={savedModule?.value.type === 'branchone'
						? savedModule.value.branches[branchIndex]?.modules[index]
						: undefined}
					{enableAi}
				/>
			{/each}
		{/if}
	{/each}
{:else if flowModule.value.type === 'branchall'}
	{#each flowModule.value.branches as branch, branchIndex (branchIndex)}
		{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchAllWrapper {noEditor} bind:branch={flowModule.value.branches[branchIndex]} />
		{:else}
			{#each branch.modules as _, index}
				<FlowModuleWrapper
					{noEditor}
					bind:flowModule={flowModule.value.branches[branchIndex].modules[index]}
					bind:parentModule={flowModule}
					previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					{enableAi}
					savedModule={savedModule?.value.type === 'branchall'
						? savedModule.value.branches[branchIndex]?.modules[index]
						: undefined}
				/>
			{/each}
		{/if}
	{/each}
{/if}
