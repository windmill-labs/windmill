<script lang="ts">
	import { Script, type FlowModule } from '$lib/gen'
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
	} from '$lib/components/flows/flowStateUtils'
	import FlowInputs from './FlowInputs.svelte'
	import { flowStateStore } from '../flowState'
	import { Alert } from '$lib/components/common'
	import FlowInputsFlow from './FlowInputsFlow.svelte'
	import FlowBranchesAllWrapper from './FlowBranchesAllWrapper.svelte'
	import FlowBranchesOneWrapper from './FlowBranchesOneWrapper.svelte'

	const { selectedId, schedule } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	// These pointers are used to easily access previewArgs of parent module, and previous module
	// Pointer to parent module, only defined within Branches or Loops.
	export let parentModule: FlowModule | undefined = undefined
	// Pointer to previous module, for easy access to testing results
	export let previousModule: FlowModule | undefined = undefined
</script>

{#key $selectedId}
	{#if flowModule.id === $selectedId}
		{#if flowModule.value.type === 'forloopflow'}
			<FlowLoop bind:mod={flowModule} {parentModule} {previousModule} />
		{:else if flowModule.value.type === 'branchone'}
			<FlowBranchesOneWrapper {previousModule} bind:flowModule />
		{:else if flowModule.value.type === 'branchall'}
			<FlowBranchesAllWrapper {previousModule} bind:flowModule />
		{:else if flowModule.value.type === 'identity'}
			{#if $selectedId == 'failure'}
				<Alert type="info" title="Error handlers are triggered upon non recovered errors">
					If defined, the error handler will take as input, the result of the step that errored
					(which has its error in the 'error field').
					<br />
					<br />
					Steps are retried until they succeed, or until the maximum number of retries defined for that
					spec is reached, at which point the error handler is called.
				</Alert>
			{/if}

			{#if flowModule.value.flow}
				<FlowInputsFlow
					failureModule={$selectedId === 'failure'}
					on:pick={async ({ detail }) => {
						const { path, summary } = detail
						const [module, state] = await pickFlow(path, summary, flowModule.id)

						flowModule = module
						$flowStateStore[module.id] = state
					}}
				/>
			{:else}
				<FlowInputs
					summary={flowModule.summary}
					shouldDisableTriggerScripts={parentModule !== undefined ||
						previousModule !== undefined ||
						$selectedId == 'failure'}
					on:pick={async ({ detail }) => {
						const { path, summary, kind, hash } = detail
						const [module, state] = await pickScript(path, summary, flowModule.id, hash)

						if (kind == Script.kind.APPROVAL) {
							module.suspend = { required_events: 1, timeout: 1800 }
						}

						if (kind == Script.kind.TRIGGER) {
							if (!$schedule.cron) {
								$schedule.cron = '0 */15 * * *'
							}
							$schedule.enabled = true

							module.stop_after_if = {
								expr: 'result == undefined || Array.isArray(result) && result.length == 0',
								skip_if_stopped: true
							}
						}

						flowModule = module
						$flowStateStore[module.id] = state
					}}
					on:new={async ({ detail }) => {
						const { language, kind, subkind } = detail

						const [module, state] = await createInlineScriptModule(
							language,
							kind,
							subkind,
							flowModule.id
						)

						if (kind == Script.kind.TRIGGER) {
							if (!$schedule.cron) {
								$schedule.cron = '0 */15 * * *'
							}
							$schedule.enabled = true

							module.stop_after_if = {
								expr: 'result == undefined || Array.isArray(result) && result.length == 0',
								skip_if_stopped: true
							}
						}

						if (kind == Script.kind.APPROVAL) {
							module.suspend = { required_events: 1, timeout: 1800 }
						}

						flowModule = module
						$flowStateStore[module.id] = state
					}}
					failureModule={$selectedId === 'failure'}
				/>
			{/if}
		{:else if flowModule.value.type === 'rawscript' || flowModule.value.type === 'script' || flowModule.value.type === 'flow'}
			<FlowModuleComponent
				bind:flowModule
				{parentModule}
				{previousModule}
				failureModule={$selectedId === 'failure'}
			/>
		{/if}
	{:else if flowModule.value.type === 'forloopflow'}
		{#each flowModule.value.modules as submodule, index (index)}
			<svelte:self
				bind:flowModule={submodule}
				bind:parentModule={flowModule}
				previousModule={flowModule.value.modules[index - 1]}
			/>
		{/each}
	{:else if flowModule.value.type === 'branchone'}
		{#if $selectedId === `${flowModule?.id}-branch-default`}
			<div class="p-2">
				<h3 class="mb-4">Default branch</h3>
				Nothing to configure, this is the default branch if none of the predicates are met.
			</div>
		{:else}
			{#each flowModule.value.default as submodule, index}
				<svelte:self
					bind:flowModule={submodule}
					bind:parentModule={flowModule}
					previousModule={flowModule.value.default[index - 1]}
				/>
			{/each}
		{/if}
		{#each flowModule.value.branches as branch, branchIndex (branchIndex)}
			{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
				<FlowBranchOneWrapper bind:branch parentModule={flowModule} {previousModule} />
			{:else}
				{#each branch.modules as submodule, index}
					<svelte:self
						bind:flowModule={submodule}
						bind:parentModule={flowModule}
						previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					/>
				{/each}
			{/if}
		{/each}
	{:else if flowModule.value.type === 'branchall'}
		{#each flowModule.value.branches as branch, branchIndex (branchIndex)}
			{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
				<FlowBranchAllWrapper bind:branch />
			{:else}
				{#each branch.modules as submodule, index}
					<svelte:self
						bind:flowModule={submodule}
						bind:parentModule={flowModule}
						previousModule={flowModule.value.branches[branchIndex].modules[index - 1]}
					/>
				{/each}
			{/if}
		{/each}
	{/if}
{/key}
