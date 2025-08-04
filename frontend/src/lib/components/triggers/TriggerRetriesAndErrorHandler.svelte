<script lang="ts">
	import type { ErrorHandler, Retry } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { emptyString } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import ErrorOrRecoveryHandler from '../ErrorOrRecoveryHandler.svelte'
	import FlowRetries from '../flows/content/FlowRetries.svelte'
	import Tooltip from '../Tooltip.svelte'

	let {
		optionTabSelected,
		itemKind,
		can_write,
		errorHandlerSelected = $bindable(),
		error_handler_path = $bindable(),
		error_handler_args = $bindable(),
		email_recipients = $bindable([]),
		retry = $bindable()
	}: {
		optionTabSelected: 'error_handler' | 'retries' | string
		itemKind: 'script' | 'flow'
		can_write: boolean
		errorHandlerSelected: ErrorHandler
		error_handler_path: string | undefined
		error_handler_args: Record<string, any>
		retry: Retry | undefined
		email_recipients: string[] | undefined
	} = $props()
</script>

{#if ['error_handler', 'retries'].includes(optionTabSelected) && itemKind !== 'script'}
	<Alert type="info" title="Only available for scripts" class="mb-2">
		Error Handler and Retries are only available for scripts. For flows, use the built-in <a
			href="https://www.windmill.dev/docs/flows/flow_error_handler"
			target="_blank">error handler</a
		>
		and <a href="https://www.windmill.dev/docs/flows/retries" target="_blank">retries</a>.
	</Alert>
{:else if optionTabSelected === 'error_handler'}
	<ErrorOrRecoveryHandler
		isEditable={can_write}
		errorOrRecovery="error"
		showScriptHelpText={true}
		bind:handlerSelected={errorHandlerSelected}
		bind:handlerPath={error_handler_path}
		bind:emailRecipients={email_recipients}
		toggleText="Alert channel on error"
		customScriptTemplate="/scripts/add?hub=hub%2F13953%2Fwindmill%2Ftrigger_error_handler_template"
		customHandlerKind="script"
		bind:handlerExtraArgs={error_handler_args}
	>
		{#snippet customTabTooltip()}
			<Tooltip>
				<div class="flex gap-20 items-start mt-3">
					<div class="text-sm"
						>The following args will be passed to the error handler:
						<ul class="mt-1 ml-2">
							<li><b>workspace_id</b>: The ID of the workspace that the trigger belongs to.</li>
							<li><b>job_id</b>: The UUID of the job that errored.</li>
							<li><b>path</b>: The path of the script or flow that failed.</li>
							<li><b>is_flow</b>: Whether the runnable is a flow.</li>
							<li
								><b>trigger_path</b>: The path of the trigger in the format
								trigger_type/trigger_path.</li
							>
							<li><b>error</b>: The error details.</li>
							<li><b>started_at</b>: The start datetime of the latest job that failed.</li>
						</ul>
					</div>
				</div>
			</Tooltip>
		{/snippet}</ErrorOrRecoveryHandler
	>
{:else if optionTabSelected === 'retries'}
	{@const disabled = !can_write || emptyString($enterpriseLicense)}
	<FlowRetries bind:flowModuleRetry={retry} {disabled} />
{/if}
