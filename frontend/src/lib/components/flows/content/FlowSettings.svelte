<script lang="ts">
	import { run } from 'svelte/legacy'

	import Path from '$lib/components/Path.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import JobTokenScopesSetting from '$lib/components/JobTokenScopesSetting.svelte'
	import { Alert, Tab, TabContent, Tabs } from '$lib/components/common'
	import { getContext, tick } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SettingGroup from '$lib/components/itemSettings/SettingGroup.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import AIFormSettings from '$lib/components/copilot/AIFormSettings.svelte'
	import DebounceLimit from '../DebounceLimit.svelte'
	import SettingToggle from '$lib/components/itemSettings/SettingToggle.svelte'
	import WorkerTagSetting from '$lib/components/itemSettings/WorkerTagSetting.svelte'
	import ConcurrencyLimitSetting from '$lib/components/itemSettings/ConcurrencyLimitSetting.svelte'
	import CacheSetting from '$lib/components/itemSettings/CacheSetting.svelte'
	import PrioritySetting from '$lib/components/itemSettings/PrioritySetting.svelte'
	import DedicatedWorkersSetting from '$lib/components/itemSettings/DedicatedWorkersSetting.svelte'
	import DeleteAfterUseSetting from '$lib/components/itemSettings/DeleteAfterUseSetting.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import InvisibleRunsSetting from '$lib/components/itemSettings/InvisibleRunsSetting.svelte'
	import SettingIcons from '$lib/components/itemSettings/SettingIcons.svelte'
	import ActiveSettingChips, {
		revealSetting
	} from '$lib/components/itemSettings/ActiveSettingChips.svelte'
	import {
		flowRuntimeFields,
		getActivePermissionBadges,
		getActiveScriptSettingsBadges
	} from '$lib/components/scriptSettings'
	import { modulesWithRetryOrSleep, SAME_WORKER_INCOMPATIBLE_MSG } from '../utils.svelte'
	import { earlyReturnCandidates } from '../earlyReturnCandidates'
	import Select from '$lib/components/select/Select.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		noEditor: boolean
		enableAi?: boolean
		/** The `settings-…` selection id the panel was opened with. */
		selectedId?: string
		/** Never deployed, though it may already have a path: a draft-only flow. */
		newFlow?: boolean
	}

	let { noEditor, enableAi, selectedId, newFlow = false }: Props = $props()

	const {
		flowStore,
		initialPathStore,
		previewArgs,
		pathStore,
		customUi,
		savedOnBehalfOfEmail,
		savedOnBehalfOfPermissionedAs,
		opWorkspace
	} = getContext<FlowEditorContext>('FlowEditorContext')

	type FlowSettingsTab = 'metadata' | 'runtime' | 'permissions'

	// Other panels open the settings on one setting by selecting `settings-<setting>`; the value
	// is the row's `data-setting`.
	const RUNTIME_ROW_OF_SELECTION: Record<string, string> = {
		'settings-worker-group': 'worker-group',
		'settings-concurrency': 'concurrency',
		'settings-cache': 'cache',
		'settings-early-stop': 'early-stop',
		'settings-early-return': 'early-return',
		'settings-same-worker': 'same-worker'
	}
	function tabOf(id: string | undefined): FlowSettingsTab {
		if (id === 'settings-runtime' || (id && id in RUNTIME_ROW_OF_SELECTION)) return 'runtime'
		return 'metadata'
	}
	// Follows the selection, and a click on a tab overrides it until the next one.
	let tab: FlowSettingsTab = $derived(tabOf(selectedId))

	function scrollToSelectedRow(node: HTMLElement) {
		const row = selectedId ? RUNTIME_ROW_OF_SELECTION[selectedId] : undefined
		if (row) node.querySelector(`[data-setting="${row}"]`)?.scrollIntoView({ block: 'nearest' })
	}

	let runtimeBadges = $derived(getActiveScriptSettingsBadges(flowRuntimeFields(flowStore.val)))
	let permissionBadges = $derived(getActivePermissionBadges(flowStore.val))
	let tabsContent: HTMLElement | undefined = $state(undefined)
	async function openSetting(target: string, setting: string) {
		tab = target as FlowSettingsTab
		await tick()
		revealSetting(tabsContent, setting)
	}

	function asSchema(x: any) {
		return x as Schema
	}
	let path: Path | undefined = $state(undefined)
	let dirtyPath = $state(false)

	// Only block turning it on: a flow deployed with both (CLI, YAML editor) must stay fixable.
	let conflictingModuleIds = $derived(
		flowStore.val.value.same_worker ? [] : modulesWithRetryOrSleep(flowStore.val.value)
	)

	run(() => {
		// preserve_step_tags has no effect without a flow worker tag; clear it so it doesn't
		// linger as invisible state when the tag is removed via any path (the picker, the
		// toggle, or the YAML editor).
		if (!flowStore.val.tag && flowStore.val.value.preserve_step_tags) {
			flowStore.val.value.preserve_step_tags = undefined
		}
	})
</script>

{#snippet sameWorkerConflict()}
	{SAME_WORKER_INCOMPATIBLE_MSG} Remove them from step{conflictingModuleIds.length > 1 ? 's' : ''}
	{conflictingModuleIds.join(', ')} first.
{/snippet}

<div class="h-full flex flex-col">
	<FlowCard {noEditor}>
		{#snippet header()}
			<!-- The title is drawn here rather than through `title` so the chips sit beside it. -->
			<div class="mr-auto flex min-w-0 items-center gap-3 pr-2">
				<div class="shrink-0 text-sm font-semibold text-emphasis">Settings</div>
				<ActiveSettingChips
					groups={[
						{ tab: 'runtime', badges: runtimeBadges },
						{ tab: 'permissions', badges: permissionBadges }
					]}
					onopen={openSetting}
				/>
			</div>
		{/snippet}
		<div class="flex flex-col h-full">
			<Tabs bind:selected={tab} wrapperClass="flex-none w-full">
				<Tab value="metadata" label="Metadata" />
				<Tab value="runtime" label="Runtime">
					{#snippet extra()}
						<SettingIcons badges={runtimeBadges} />
					{/snippet}
				</Tab>
				<Tab value="permissions" label="Permissions">
					{#snippet extra()}
						<SettingIcons badges={permissionBadges} />
					{/snippet}
				</Tab>

				{#snippet content()}
					<div class="min-h-0 grow overflow-y-auto" bind:this={tabsContent}>
						<TabContent value="metadata">
							<div class="flex flex-col gap-6 p-4 pb-8">
								<Label label="Summary">
									<MetadataGen
										bind:content={flowStore.val.summary}
										promptConfigName="flowSummary"
										flow={flowStore.val.value}
										onChange={() => {
											if (
												$initialPathStore == '' &&
												flowStore.val.summary?.length > 0 &&
												!dirtyPath
											) {
												path?.setName(
													flowStore.val.summary
														.toLowerCase()
														.replace(/[^a-z0-9_]/g, '_')
														.replace(/-+/g, '_')
														.replace(/^-|-$/g, '')
												)
											}
										}}
										elementProps={{
											type: 'text',
											id: 'flow-summary',
											placeholder: 'Short summary to be displayed when listed'
										}}
									/>
								</Label>
								<!-- prettier-ignore -->
								<LabelsInput bind:labels={(flowStore.val as any).labels} class="-mt-4" />

								{#if !noEditor && customUi?.topBar?.editablePath != false}
									<Label label="Path">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions#path"
											>
												The unique identifier of the flow in the workspace that defines permissions
											</Tooltip>
										{/snippet}
										<Path
											autofocus={false}
											bind:this={path}
											bind:dirty={dirtyPath}
											bind:path={$pathStore}
											initialPath={$initialPathStore}
											namePlaceholder="flow"
											kind="flow"
											workspaceOverride={opWorkspace?.()}
										/>
										{#if $initialPathStore && $pathStore && $pathStore !== $initialPathStore}
											<Alert
												type="info"
												size="xs"
												title="Deploy the flow to make the path change effective."
												class="mt-2"
											/>
										{/if}
									</Label>
								{/if}

								<Label label="Description">
									<MetadataGen
										bind:content={flowStore.val.description}
										promptConfigName="flowDescription"
										flow={flowStore.val.value}
										class="w-full"
										elementType="textarea"
										elementProps={{
											id: 'inp',
											placeholder: 'What this flow does and how to use it'
										}}
									/>
								</Label>

								{#if flowStore.val.schema && enableAi}
									<AIFormSettings
										bind:prompt={flowStore.val.schema.prompt_for_ai as string | undefined}
										type="flow"
									/>
								{/if}
							</div>
						</TabContent>

						<!-- The rows, their wording and order are the script Runtime tab's; Flow control is flow-only. -->
						<TabContent value="runtime">
							<div class="flex flex-col gap-8 p-4 pb-8" {@attach scrollToSelectedRow}>
								{#if customUi?.settingsTabs?.earlyStop != false || customUi?.settingsTabs?.earlyReturn != false}
									<SettingGroup label="Flow control" class="flex flex-col gap-6">
										{#if customUi?.settingsTabs?.earlyStop != false}
											<SettingToggle
												setting="early-stop"
												label="Stop flow if"
												tooltip="Evaluated on the flow inputs before the first step. When true, the flow stops right away and returns 'stopped early'."
												documentationLink="https://www.windmill.dev/docs/flows/early_stop#early-stop-for-flow"
												checked={!!flowStore.val.value.skip_expr}
												onchange={(on) =>
													(flowStore.val.value.skip_expr = on
														? 'flow_input.foo == undefined'
														: undefined)}
											>
												<div class="w-full border rounded-md overflow-auto">
													<SimpleEditor
														lang="javascript"
														small
														bind:code={
															() => flowStore.val.value.skip_expr,
															(code) => {
																// The editor writes its text back when it loses focus, which can
																// land while the row closes and would turn the setting back on.
																if (flowStore.val.value.skip_expr !== undefined) {
																	flowStore.val.value.skip_expr = code
																}
															}
														}
														class="few-lines-editor"
														extraLib={`declare const flow_input = ${JSON.stringify(
															schemaToObject(asSchema(flowStore.val.schema), previewArgs.val)
														)};
												declare const WM_SCHEDULED_FOR: string;`}
													/>
												</div>
												<div class="text-2xs text-secondary">
													<code>flow_input</code> holds the flow inputs and
													<code>WM_SCHEDULED_FOR</code> the time the run was scheduled for, e.g. to
													stop runs that started too late:
													<code
														>new Date().getTime() - new Date(WM_SCHEDULED_FOR).getTime() {'>'} X</code
													>
												</div>
											</SettingToggle>
										{/if}
										{#if customUi?.settingsTabs?.earlyReturn != false}
											<SettingToggle
												setting="early-return"
												label="Early return for sync webhooks"
												tooltip="Sync endpoints return the result of the step picked here while the rest of the flow keeps running."
												documentationLink="https://www.windmill.dev/docs/flows/early_return"
												checked={!!flowStore.val.value.early_return}
												onchange={(on) =>
													(flowStore.val.value.early_return = on
														? (flowStore.val.value.modules?.[0]?.id ?? 'a')
														: undefined)}
											>
												{@const candidates = earlyReturnCandidates(
													flowStore.val.value.modules ?? []
												)}
												{@const selected = candidates.find(
													(c) => c.id === flowStore.val.value.early_return
												)}
												<div class="max-w-sm">
													<Select
														items={candidates.map((c) => ({ value: c.id, label: c.id }))}
														bind:value={flowStore.val.value.early_return}
														placeholder="Node's id"
													/>
												</div>
												{#if !selected}
													<span class="text-2xs text-red-600 dark:text-red-400">
														Step {flowStore.val.value.early_return} is not one of the steps above. Pick
														another step.
													</span>
												{:else if !selected.deterministic}
													<span class="text-2xs text-secondary">
														This step only runs if its branch is taken. Otherwise, sync calls return
														the flow's final result.
													</span>
												{/if}
											</SettingToggle>
										{/if}
									</SettingGroup>
								{/if}

								<SettingGroup label="Execution policy" class="flex flex-col gap-6">
									{#if customUi?.settingsTabs?.concurrency != false}
										<ConcurrencyLimitSetting
											kind="flow"
											bind:limit={flowStore.val.value.concurrent_limit}
											bind:timeWindowS={flowStore.val.value.concurrency_time_window_s}
											bind:concurrencyKey={flowStore.val.value.concurrency_key}
											keyPlaceholder={`$workspace/flow/${$pathStore}-$args[foo]`}
										/>
									{/if}
									{#if customUi?.settingsTabs?.debouncing != false}
										<DebounceLimit
											bind:debounce_delay_s={flowStore.val.value.debounce_delay_s}
											bind:debounce_key={flowStore.val.value.debounce_key}
											bind:debounce_args_to_accumulate={
												flowStore.val.value.debounce_args_to_accumulate
											}
											bind:max_total_debouncing_time={flowStore.val.value.max_total_debouncing_time}
											bind:max_total_debounces_amount={
												flowStore.val.value.max_total_debounces_amount
											}
											schema={asSchema(flowStore.val.schema)}
											placeholder={`$workspace/flow/${$pathStore}-$args[foo]`}
										/>
									{/if}
									{#if customUi?.settingsTabs?.cache != false}
										<CacheSetting
											kind="flow"
											bind:ttl={flowStore.val.value.cache_ttl}
											bind:ignoreS3Path={flowStore.val.value.cache_ignore_s3_path}
										/>
									{/if}
									<PrioritySetting kind="flow" bind:priority={flowStore.val.value.priority} />
									<DeleteAfterUseSetting
										kind="flow"
										bind:deleteAfterSecs={flowStore.val.value.delete_after_secs}
									/>
								</SettingGroup>

								<SettingGroup label="Workers" class="flex flex-col gap-6">
									{#if customUi?.settingsTabs?.workerGroup != false}
										<WorkerTagSetting
											kind="flow"
											bind:tag={flowStore.val.tag}
											bind:preserveStepTags={flowStore.val.value.preserve_step_tags}
											workspaceId={opWorkspace?.()}
										/>
									{/if}
									<DedicatedWorkersSetting
										kind="flow"
										bind:dedicatedWorker={flowStore.val.dedicated_worker}
									/>
									{#if customUi?.settingsTabs?.sharedDiretory != false}
										<SettingToggle
											setting="same-worker"
											label="Shared directory"
											tooltip="Every step runs on the same worker and shares a `./shared` folder to pass heavier data to the next step. The folder is not kept across suspends and sleeps."
											documentationLink="https://www.windmill.dev/docs/core_concepts/persistent_storage/within_windmill#shared-directory"
											disabled={conflictingModuleIds.length > 0}
											checked={!!flowStore.val.value.same_worker}
											onchange={(on) => (flowStore.val.value.same_worker = on)}
											hint={conflictingModuleIds.length > 0 ? sameWorkerConflict : undefined}
										/>
									{/if}
								</SettingGroup>
							</div>
						</TabContent>

						<TabContent value="permissions">
							<div class="flex flex-col gap-6 p-4 pb-8">
								<RunOnBehalfOfSetting
									kind="flow"
									bind:email={flowStore.val.on_behalf_of_email}
									bind:permissionedAs={flowStore.val.on_behalf_of}
									savedEmail={$savedOnBehalfOfEmail}
									savedPermissionedAs={$savedOnBehalfOfPermissionedAs}
									targetWorkspace={opWorkspace?.() ?? $operatingWorkspace ?? ''}
									isNew={newFlow || $initialPathStore == ''}
									itemPath={$pathStore}
									needsSource={() => ({
										kind: 'flow',
										value: flowStore.val.value,
										schema: flowStore.val.schema
									})}
								/>
								<InvisibleRunsSetting
									kind="flow"
									bind:visibleToRunnerOnly={flowStore.val.visible_to_runner_only}
								/>
								<JobTokenScopesSetting bind:value={flowStore.val.job_token_scopes} kind="flow" />
							</div>
						</TabContent>
					</div>
				{/snippet}
			</Tabs>
		</div>
	</FlowCard>
</div>
