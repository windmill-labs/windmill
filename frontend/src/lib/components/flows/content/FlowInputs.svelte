<script lang="ts">
	import { Alert } from '$lib/components/common'
	import ToggleHubWorkspace from '$lib/components/ToggleHubWorkspace.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import { createEventDispatcher, getContext } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import WorkspaceScriptPicker from '../pickers/WorkspaceScriptPicker.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { sendUserToast } from '$lib/toast'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Check, Code, Zap } from 'lucide-svelte'
	import SuspendDrawer from './SuspendDrawer.svelte'
	import { defaultScripts } from '$lib/stores'
	import { defaultScriptLanguages, processLangs } from '$lib/scripts'
	import type { SupportedLanguage } from '$lib/common'
	import DefaultScripts from '$lib/components/DefaultScripts.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'

	interface Props {
		failureModule: boolean
		preprocessorModule: boolean
		shouldDisableTriggerScripts?: boolean
		noEditor: boolean
		summary?: string | undefined
	}

	let {
		failureModule,
		preprocessorModule,
		shouldDisableTriggerScripts = false,
		noEditor,
		summary = $bindable(undefined)
	}: Props = $props()

	const dispatch = createEventDispatcher()
	let kind: 'script' | 'failure' | 'approval' | 'trigger' = $state(
		failureModule
			? 'failure'
			: summary == 'Trigger'
				? 'trigger'
				: summary == 'Approval'
					? 'approval'
					: 'script'
	)
	let pick_existing: 'workspace' | 'hub' = $state('hub')
	let filter = $state('')

	let langs = $derived(
		processLangs(undefined, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
			.map((l) => [defaultScriptLanguages[l], l])
			.filter(
				(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1])
			) as [string, SupportedLanguage | 'docker'][]
	)

	function displayLang(lang: SupportedLanguage | 'docker', kind: string) {
		if (lang == 'bun' || lang == 'python3' || lang == 'deno') {
			return true
		}

		if (lang == 'go') {
			return (kind == 'script' || kind == 'trigger' || failureModule) && !preprocessorModule
		}

		if (lang == 'bash' || lang == 'nativets') {
			return kind == 'script' && !preprocessorModule
		}
		return kind == 'script' && !failureModule && !preprocessorModule
	}

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
</script>

<div class="p-4 h-full flex flex-col" id="flow-editor-flow-inputs">
	{#if summary == 'Terminate flow'}
		<Alert role="info" title="The flow stops here"
			>This is an identity step with an early stop that has 'true' for expression</Alert
		>
	{:else}{#if !failureModule && !preprocessorModule}
			<div class="center-center">
				<div class="max-w-min">
					<ToggleButtonGroup bind:selected={kind}>
						{#snippet children({ item })}
							<ToggleButton
								value="script"
								icon={Code}
								label="Action"
								tooltip="An action script is simply a script that is neither a trigger nor an approval script. Those are the majority of the scripts."
								{item}
							/>
							{#if !shouldDisableTriggerScripts}
								<ToggleButton
									value="trigger"
									icon={Zap}
									label="Trigger"
									tooltip="Used as a first step most commonly with a state and a schedule to watch for changes on an external system, compute the diff since last time and set the new state. The diffs are then treated one by one with a for-loop."
									{item}
								/>
							{/if}
							<ToggleButton
								value="approval"
								icon={Check}
								label="Approval"
								tooltip="An approval step will suspend the execution of a flow until it has been approved through the resume endpoints or the approval page by and solely by the recipients of those secret urls."
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>
				</div>
			</div>
		{/if}
		{#if kind == 'trigger'}
			<div class="mt-2"></div>
			<Alert title="Trigger scripts" role="info">
				Trigger scripts are designed to pull data from an external source and return all of the new
				items since the last run, without resorting to external webhooks.<br /><br />

				A trigger script is intended to be used with
				<a
					href="https://www.windmill.dev/docs/core_concepts/scheduling"
					target="_blank"
					class="text-blue-400">schedules</a
				>
				and
				<a
					href="https://www.windmill.dev/docs/core_concepts/resources_and_types#states"
					target="_blank"
					class="text-blue-400">states</a
				>
				in order to compare the execution to the previous one and process each new item in a
				<a
					href="https://www.windmill.dev/docs/flows/flow_loops"
					target="_blank"
					class="text-blue-400">for loop</a
				>. If there are no new items, the flow will be skipped.<br /><br />

				By default, adding a trigger will set the schedule to 15 minutes. To see all ways to trigger
				a flow, check
				<a
					href="https://www.windmill.dev/docs/getting_started/triggers"
					target="_blank"
					class="text-blue-400">Triggering Flows</a
				>.
			</Alert>
		{/if}

		{#if kind == 'script' && !noEditor && !preprocessorModule}
			<div class="mt-2"></div>
			<Alert title="Action Scripts" role="info">
				An action script is simply a script that is neither a trigger nor an approval script. Those
				are the majority of the scripts.
			</Alert>
		{/if}

		{#if kind == 'approval'}
			{#if !noEditor}
				<div class="mt-2"></div>
				<Alert title="Approval/Prompt Step" role="info">
					An approval/prompt step will suspend the execution of a flow until it has been approved
					and/or the prompts have been filled in the UI or through the resume endpoints or the
					approval page by and solely by the recipients of the secret urls. See details in
					'Advanced' -> 'Suspend' settings of the step. A prompt is a specialized approval step with
					payload that can be self-approved by the caller.<br /><br />
					For further details, visit
					<a
						href="https://www.windmill.dev/docs/flows/flow_approval"
						target="_blank"
						class="text-blue-500">Approval/Prompt Steps Documentation</a
					>
					or
					<div class="inline-flex">
						<SuspendDrawer text="Approval/Step prompt helpers" />
					</div>
				</Alert>
			{:else}
				<a
					href="https://www.windmill.dev/docs/flows/flow_approval"
					target="_blank"
					class="text-blue-500">Approval/Prompt Steps Documentation</a
				>
			{/if}
		{/if}
		<h3 class="pb-2 pt-4 flex gap-x-8 flex-wrap">
			<div>
				Inline new <span class="text-blue-500 dark:text-blue-400"
					>{kind == 'script' ? 'action' : kind}</span
				>
				script
				<Tooltip
					documentationLink={kind === 'script'
						? 'https://www.windmill.dev/docs/flows/editor_components#flow-actions'
						: kind === 'trigger'
							? 'https://www.windmill.dev/docs/flows/flow_trigger'
							: kind === 'approval'
								? 'https://www.windmill.dev/docs/flows/flow_approval'
								: 'https://www.windmill.dev/docs/getting_started/flows_quickstart#flow-editor'}
				>
					Embed <span>{kind == 'script' ? 'action' : kind}</span> script directly inside a flow instead
					of saving the script into your workspace for reuse. You can always save an inline script to
					your workspace later.
				</Tooltip>
			</div>
			<DefaultScripts />
		</h3>
		{#if noEditor}
			<div
				class="py-0.5 text-2xs {summary == undefined || summary == ''
					? 'text-red-600'
					: 'text-ternary'}"
				>Pick a summary first, it will be used to create a separate file whose name will be derived
				from the summary</div
			>
			<input class="w-full" type="text" bind:value={summary} placeholder="Summary" />
			<div class="pb-2"></div>
		{/if}
		<div class="flex flex-row flex-wrap gap-2" id="flow-editor-action-script">
			{#each langs.filter((lang) => customUi?.languages == undefined || customUi?.languages?.includes(lang?.[1])) as [label, lang] (lang)}
				{#if displayLang(lang, kind)}
					<FlowScriptPicker
						id={`flow-editor-action-script-${lang}`}
						disabled={noEditor && (summary == undefined || summary == '')}
						{label}
						lang={lang == 'docker' ? 'bash' : lang}
						on:click={() => {
							if (lang == 'docker') {
								if (isCloudHosted()) {
									sendUserToast(
										'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
										true,
										[
											{
												label: 'Learn more',
												callback: () => {
													window.open('https://www.windmill.dev/docs/advanced/docker', '_blank')
												}
											}
										]
									)
									return
								}
							}
							dispatch('new', {
								language: lang == 'docker' ? 'bash' : lang,
								kind,
								subkind: lang == 'docker' ? 'docker' : preprocessorModule ? 'preprocessor' : 'flow',
								summary
							})
						}}
					/>
				{/if}
			{/each}
		</div>

		<h3 class="mb-2 mt-6"
			>Use pre-made <span class="text-blue-500 dark:text-blue-400"
				>{kind == 'script' ? 'action' : kind}</span
			> script</h3
		>
		{#if pick_existing == 'hub'}
			<PickHubScript bind:filter {kind} on:pick>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</PickHubScript>
		{:else}
			<WorkspaceScriptPicker displayLock bind:filter {kind} on:pick>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</WorkspaceScriptPicker>
		{/if}
	{/if}
</div>
