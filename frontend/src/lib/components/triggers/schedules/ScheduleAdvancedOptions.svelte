<script lang="ts" module>
	import { handlerFullPath } from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import type { ErrorHandler, Retry } from '$lib/gen'

	export type ScheduleAdvancedState = {
		errorHandlerSelected: ErrorHandler
		errorHandlerPath: string | undefined
		errorHandleritemKind: 'flow' | 'script'
		errorHandlerExtraArgs: Record<string, any>
		wsErrorHandlerMuted: boolean
		failedTimes: number
		failedExact: boolean
		recoveryHandlerSelected: ErrorHandler
		recoveryHandlerPath: string | undefined
		recoveryHandlerItemKind: 'flow' | 'script'
		recoveryHandlerExtraArgs: Record<string, any>
		recoveredTimes: number
		successHandlerSelected: ErrorHandler
		successHandlerPath: string | undefined
		successHandlerItemKind: 'flow' | 'script'
		successHandlerExtraArgs: Record<string, any>
		retry: Retry | undefined
		dynamicSkipPath: string | undefined
		tag: string | undefined
	}

	export function emptyScheduleAdvanced(): ScheduleAdvancedState {
		return {
			errorHandlerSelected: 'slack',
			errorHandlerPath: undefined,
			errorHandleritemKind: 'script',
			errorHandlerExtraArgs: {},
			wsErrorHandlerMuted: false,
			failedTimes: 1,
			failedExact: false,
			recoveryHandlerSelected: 'slack',
			recoveryHandlerPath: undefined,
			recoveryHandlerItemKind: 'script',
			recoveryHandlerExtraArgs: {},
			recoveredTimes: 1,
			successHandlerSelected: 'slack',
			successHandlerPath: undefined,
			successHandlerItemKind: 'script',
			successHandlerExtraArgs: {},
			retry: undefined,
			dynamicSkipPath: undefined,
			tag: undefined
		}
	}

	/** The schedule fields the Advanced tabs own, as the schedule API takes them. */
	export function scheduleAdvancedCfg(a: ScheduleAdvancedState) {
		return {
			on_failure: a.errorHandlerPath
				? handlerFullPath(a.errorHandlerSelected, a.errorHandleritemKind, a.errorHandlerPath)
				: undefined,
			on_failure_times: a.failedTimes,
			on_failure_exact: a.failedExact,
			on_failure_extra_args: a.errorHandlerPath ? a.errorHandlerExtraArgs : undefined,
			on_recovery: a.recoveryHandlerPath
				? handlerFullPath(a.recoveryHandlerSelected, a.recoveryHandlerItemKind, a.recoveryHandlerPath)
				: undefined,
			on_recovery_times: a.recoveredTimes,
			on_recovery_extra_args: a.recoveryHandlerPath ? a.recoveryHandlerExtraArgs : {},
			on_success: a.successHandlerPath
				? handlerFullPath(a.successHandlerSelected, a.successHandlerItemKind, a.successHandlerPath)
				: undefined,
			on_success_extra_args: a.successHandlerPath ? a.successHandlerExtraArgs : {},
			ws_error_handler_muted: a.wsErrorHandlerMuted,
			retry: a.retry,
			tag: a.tag,
			dynamic_skip: a.dynamicSkipPath
		}
	}
</script>

<script lang="ts">
	import { Alert, Button, Tab, Tabs } from '$lib/components/common'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import FlowRetries from '$lib/components/flows/content/FlowRetries.svelte'
	import { ScheduleService } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { Loader2, Save } from 'lucide-svelte'

	// The schedule drawer's Advanced tabs, shared with editors that configure a
	// schedule before it exists (the pipeline editor's create wizard).
	interface Props {
		wsId: string | undefined
		itemKind: 'flow' | 'script'
		canWrite: boolean
		loading?: boolean
		errorHandlerSelected: ErrorHandler
		errorHandlerPath: string | undefined
		errorHandleritemKind: 'flow' | 'script'
		errorHandlerExtraArgs: Record<string, any>
		wsErrorHandlerMuted: boolean
		failedTimes: number
		failedExact: boolean
		recoveryHandlerSelected: ErrorHandler
		recoveryHandlerPath: string | undefined
		recoveryHandlerItemKind: 'flow' | 'script'
		recoveryHandlerExtraArgs: Record<string, any>
		recoveredTimes: number
		successHandlerSelected: ErrorHandler
		successHandlerPath: string | undefined
		successHandlerItemKind: 'flow' | 'script'
		successHandlerExtraArgs: Record<string, any>
		retry: Retry | undefined
		dynamicSkipPath: string | undefined
		tag: string | undefined
	}

	let {
		wsId,
		itemKind,
		canWrite,
		loading = false,
		errorHandlerSelected = $bindable(),
		errorHandlerPath = $bindable(),
		errorHandleritemKind = $bindable(),
		errorHandlerExtraArgs = $bindable(),
		wsErrorHandlerMuted = $bindable(),
		failedTimes = $bindable(),
		failedExact = $bindable(),
		recoveryHandlerSelected = $bindable(),
		recoveryHandlerPath = $bindable(),
		recoveryHandlerItemKind = $bindable(),
		recoveryHandlerExtraArgs = $bindable(),
		recoveredTimes = $bindable(),
		successHandlerSelected = $bindable(),
		successHandlerPath = $bindable(),
		successHandlerItemKind = $bindable(),
		successHandlerExtraArgs = $bindable(),
		retry = $bindable(),
		dynamicSkipPath = $bindable(),
		tag = $bindable()
	}: Props = $props()

	let optionTabSelected:
		| 'error_handler'
		| 'recovery_handler'
		| 'success_handler'
		| 'retries'
		| 'dynamic_skip'
		| 'tag' = $state('error_handler')

	// Carry the acting workspace onto "create from template" routes, so the script is created
	// in the workspace this schedule lives in.
	const wsParam = $derived(wsId ? `&workspace=${encodeURIComponent(wsId)}` : '')

	async function saveAsDefaultErrorHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default error handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'error',
					override_existing: overrideExisting,
					path:
						errorHandlerPath == undefined
							? undefined
							: handlerFullPath(errorHandlerSelected, errorHandleritemKind, errorHandlerPath),
					extra_args: errorHandlerExtraArgs,
					number_of_occurence: failedTimes,
					number_of_occurence_exact: failedExact,
					workspace_handler_muted: wsErrorHandlerMuted
				}
			})
			if (errorHandlerPath !== undefined) {
				sendUserToast(`Default error handler saved to ${errorHandlerPath}`, false)
			} else {
				sendUserToast(`Default error handler reset`, false)
			}
		}
	}

	async function saveAsDefaultRecoveryHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default recovery handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'recovery',
					override_existing: overrideExisting,
					path:
						recoveryHandlerPath === undefined
							? undefined
							: handlerFullPath(
									recoveryHandlerSelected,
									recoveryHandlerItemKind,
									recoveryHandlerPath
								),
					extra_args: recoveryHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (recoveryHandlerPath !== undefined) {
				sendUserToast(`Default recovery handler saved to ${recoveryHandlerPath}`, false)
			} else {
				sendUserToast(`Default recovery handler reset`, false)
			}
		}
	}

	async function saveAsDefaultSuccessHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default success handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'success',
					override_existing: overrideExisting,
					path:
						successHandlerPath === undefined
							? undefined
							: handlerFullPath(successHandlerSelected, successHandlerItemKind, successHandlerPath),
					extra_args: successHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (successHandlerPath !== undefined) {
				sendUserToast(`Default success handler saved to ${successHandlerPath}`, false)
			} else {
				sendUserToast(`Default success handler reset`, false)
			}
		}
	}
</script>

<div class="flex flex-col gap-2 min-h-96">
	{#if !loading}
		<Tabs bind:selected={optionTabSelected}>
			<Tab value="error_handler" label="Error Handler" />
			<Tab value="recovery_handler" label="Recovery Handler" />
			<Tab value="success_handler" label="Success Handler" />
			<Tab value="retries" label="Retries" />
			<Tab value="dynamic_skip" label="Dynamic skip" />
			{#if itemKind === 'script'}
				<Tab value="tag" label="Custom tag" />
			{/if}
		</Tabs>
		{#if optionTabSelected === 'error_handler'}
			<Section label="Error handler">
				{#snippet header()}
					<div class="flex flex-row gap-2">
						{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
					</div>
				{/snippet}
				{#snippet action()}
					<div class="flex flex-row items-center gap-1 text-xs text-secondary">
						<Dropdown
							disabled={!canWrite}
							items={[
								{
									displayName: `Override future schedules only`,
									action: () => saveAsDefaultErrorHandler(false)
								},
								{
									displayName: 'Override all existing',
									type: 'delete',
									action: () => saveAsDefaultErrorHandler(true)
								}
							]}
						>
							{#snippet buttonReplacement()}
								<Save size={12} class="mr-1" />
								Set as default
							{/snippet}
						</Dropdown>
					</div>
				{/snippet}
				<div class="flex flex-row py-2">
					<Toggle
						size="xs"
						disabled={!canWrite || !$enterpriseLicense}
						bind:checked={wsErrorHandlerMuted}
						options={{ right: 'Mute workspace error handler for this schedule' }}
					/>
				</div>

				<ErrorOrRecoveryHandler
					workspace={wsId}
					isEditable={canWrite}
					errorOrRecovery="error"
					showScriptHelpText={true}
					bind:handlerSelected={errorHandlerSelected}
					bind:handlerPath={errorHandlerPath}
					toggleText="Alert channel on error"
					customScriptTemplate="/scripts/add?hub=hub%2F19743%2Fwindmill%2Fschedule_error_handler_template"
					bind:customHandlerKind={errorHandleritemKind}
					bind:handlerExtraArgs={errorHandlerExtraArgs}
				>
					{#snippet customTabTooltip()}
						<Tooltip>
							<div class="flex gap-20 items-start mt-3">
								<div class="text-xs"
									>The following args will be passed to the error handler:
									<ul class="mt-1 ml-2">
										<li
											><b>workspace_id</b>: The ID of the workspace that the schedule belongs to.</li
										>
										<li><b>job_id</b>: The UUID of the job that errored.</li>
										<li><b>path</b>: The path of the script or flow that failed.</li>
										<li><b>is_flow</b>: Whether the runnable is a flow.</li>
										<li><b>schedule_path</b>: The path of the schedule.</li>
										<li><b>error</b>: The error details.</li>
										<li
											><b>failed_times</b>: Minimum number of times the schedule failed before
											calling the error handler.</li
										>
										<li><b>started_at</b>: The start datetime of the latest job that failed.</li>
									</ul>
								</div>
							</div>
						</Tooltip>
					{/snippet}
				</ErrorOrRecoveryHandler>
				<div class="flex flex-row items-center justify-between">
					<div class="flex flex-row items-center mt-4 font-semibold text-xs gap-2">
						<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}>
							Triggered when schedule failed</p
						>
						<select
							class="!w-14"
							bind:value={failedExact}
							disabled={!$enterpriseLicense || emptyString(errorHandlerPath)}
						>
							<option value={false}>&gt;=</option>
							<option value={true}>==</option>
						</select>
						<input
							type="number"
							class="!w-14 text-center {emptyString(errorHandlerPath) ? 'text-primary' : ''}"
							bind:value={failedTimes}
							disabled={!$enterpriseLicense}
							min="1"
						/>
						<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}
							>time{failedTimes > 1 ? 's in a row' : ''}</p
						>
					</div>
				</div>
			</Section>
		{:else if optionTabSelected === 'recovery_handler'}
			{@const disabled = !canWrite || emptyString($enterpriseLicense)}
			<Section label="Recovery handler">
				{#snippet header()}
					<div class="flex flex-row gap-2">
						{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
					</div>
				{/snippet}
				{#snippet action()}
					<div class="flex flex-row items-center text-secondary text-xs gap-2">
						defaults
						<Dropdown
							{disabled}
							items={[
								{
									displayName: `Override future schedules only`,
									action: () => saveAsDefaultRecoveryHandler(false)
								},
								{
									displayName: 'Override all existing',
									type: 'delete',
									action: () => saveAsDefaultRecoveryHandler(true)
								}
							]}
						>
							{#snippet buttonReplacement()}
								<Save size={12} class="mr-1" />
								Set as default
							{/snippet}
						</Dropdown>
					</div>
				{/snippet}
				<ErrorOrRecoveryHandler
					workspace={wsId}
					isEditable={!disabled}
					errorOrRecovery="recovery"
					bind:handlerSelected={recoveryHandlerSelected}
					bind:handlerPath={recoveryHandlerPath}
					toggleText="Alert channel when error recovered"
					customScriptTemplate="/scripts/add?hub=hub%2F9082%2Fwindmill%2Fschedule_recovery_handler_template"
					bind:customHandlerKind={recoveryHandlerItemKind}
					bind:handlerExtraArgs={recoveryHandlerExtraArgs}
				>
					{#snippet customTabTooltip()}
						<Tooltip>
							<div class="flex gap-20 items-start mt-3">
								<div class="text-xs"
									>The following args will be passed to the recovery handler:
									<ul class="mt-1 ml-2">
										<li><b>path</b>: The path of the script or flow that recovered.</li>
										<li><b>is_flow</b>: Whether the runnable is a flow.</li>
										<li><b>schedule_path</b>: The path of the schedule.</li>
										<li><b>error</b>: The error of the last job that errored</li>
										<li
											><b>error_started_at</b>: The start datetime of the last job that errored</li
										>
										<li
											><b>success_times</b>: The number of times the schedule succeeded before
											calling the recovery handler.</li
										>
										<li><b>success_result</b>: The result of the latest successful job</li>
										<li
											><b>success_started_at</b>: The start datetime of the latest successful job</li
										>
									</ul>
								</div>
							</div>
						</Tooltip>
					{/snippet}
				</ErrorOrRecoveryHandler>
				<div class="flex flex-row items-center justify-between">
					<div
						class="flex flex-row items-center mt-5 font-semibold text-xs {emptyString(
							recoveryHandlerPath
						)
							? 'text-primary'
							: ''}"
					>
						<p>Triggered when schedule recovered</p>
						<input
							type="number"
							class="!w-14 mx-2 text-center"
							bind:value={recoveredTimes}
							min="1"
							{disabled}
						/>
						<p>time{recoveredTimes > 1 ? 's in a row' : ''}</p>
					</div>
				</div>
			</Section>
		{:else if optionTabSelected === 'success_handler'}
			{@const disabled = !canWrite || emptyString($enterpriseLicense)}
			<Section label="Success handler">
				{#snippet header()}
					<div class="flex flex-row gap-2">
						{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
					</div>
				{/snippet}
				{#snippet action()}
					<div class="flex flex-row items-center text-secondary text-xs gap-2">
						defaults
						<Dropdown
							{disabled}
							items={[
								{
									displayName: `Override future schedules only`,
									action: () => saveAsDefaultSuccessHandler(false)
								},
								{
									displayName: 'Override all existing',
									type: 'delete',
									action: () => saveAsDefaultSuccessHandler(true)
								}
							]}
						>
							{#snippet buttonReplacement()}
								<Save size={12} class="mr-1" />
								Set as default
							{/snippet}
						</Dropdown>
					</div>
				{/snippet}
				<ErrorOrRecoveryHandler
					workspace={wsId}
					isEditable={!disabled}
					errorOrRecovery="success"
					bind:handlerSelected={successHandlerSelected}
					bind:handlerPath={successHandlerPath}
					toggleText="Alert channel when successful"
					customScriptTemplate="/scripts/add?hub=hub%2F9071%2Fwindmill%2Fschedule_success_handler_template"
					bind:customHandlerKind={successHandlerItemKind}
					bind:handlerExtraArgs={successHandlerExtraArgs}
				>
					{#snippet customTabTooltip()}
						<Tooltip>
							<div class="flex gap-20 items-start mt-3">
								<div class="text-xs"
									>The following args will be passed to the success handler:
									<ul class="mt-1 ml-2">
										<li><b>path</b>: The path of the script or flow that succeeded.</li>
										<li><b>is_flow</b>: Whether the runnable is a flow.</li>
										<li><b>schedule_path</b>: The path of the schedule.</li>
										<li><b>success_result</b>: The result of the successful job</li>
										<li><b>success_started_at</b>: The start datetime of the successful job</li>
									</ul>
								</div>
							</div>
						</Tooltip>
					{/snippet}
				</ErrorOrRecoveryHandler>
			</Section>
		{:else if optionTabSelected === 'retries'}
			{@const disabled = !canWrite || emptyString($enterpriseLicense)}
			<Section label="Retries">
				{#snippet header()}
					<div class="flex flex-row gap-2">
						{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
					</div>
					<Tooltip>
						If defined, upon error this schedule will be retried with a delay and a maximum number
						of attempts as defined below.
						<br />
						This is only available for individual script. For flows, retries can be set on each flow
						step in the flow editor.
					</Tooltip>
				{/snippet}
				{#if itemKind !== 'script'}
					<Alert type="info" title="Only available for scripts" class="mb-2">
						Error Handler and Retries are only available for scripts. For flows, use the built-in <a
							href="https://www.windmill.dev/docs/flows/flow_error_handler"
							target="_blank">error handler</a
						>
						and <a href="https://www.windmill.dev/docs/flows/retries" target="_blank">retries</a>.
					</Alert>
				{:else}
					<FlowRetries
						bind:flowModuleRetry={retry}
						disabled={itemKind !== 'script' || disabled}
					/>
				{/if}
			</Section>
		{:else if optionTabSelected === 'dynamic_skip'}
			<Section label="Dynamic skip">
				{#snippet header()}
					<Tooltip>
						Optional script to filter scheduled dates. Receives the proposed datetime and returns
						boolean. True = run on this date, False = skip to next occurrence.
					</Tooltip>
				{/snippet}
				<div class="flex flex-col gap-6">
					<Label label="Dynamic skip script">
						<div class="flex flex-row">
							<ScriptPicker
								workspace={wsId}
								disabled={!canWrite}
								bind:scriptPath={dynamicSkipPath}
								kinds={['script']}
								allowRefresh={canWrite}
								clearable
							/>
							{#if !dynamicSkipPath}
								<Button
									btnClasses="ml-4 whitespace-nowrap"
									variant="default"
									size="xs"
									href="/scripts/add?hub=hub%2F19822%2Fwindmill%2Fdynamic_skip_template{wsParam}"
									disabled={!canWrite}
									target="_blank"
								>
									Create from template
								</Button>
							{/if}
						</div>
					</Label>
					<Alert type="info" size="xs" title="Handler requirements">
						Handler must return a boolean value. Return true to execute the scheduled job, false
						to skip.
					</Alert>
				</div>
			</Section>
		{:else if optionTabSelected === 'tag'}
			<Section
				label="Custom script tag"
				tooltip="When set, the script tag will be overridden by this tag"
			>
				<WorkerTagPicker
					bind:tag
					workspaceId={wsId}
					popupPlacement="top-end"
					disabled={!canWrite}
				/>
			</Section>
		{/if}
	{:else}
		<Loader2 class="animate-spin" />
	{/if}
</div>
