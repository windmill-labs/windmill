<script lang="ts">
	import type { Retry } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { emptyString } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import ErrorOrRecoveryHandler from '../ErrorOrRecoveryHandler.svelte'
	import FlowRetries from '../flows/content/FlowRetries.svelte'

	let {
		optionTabSelected,
		itemKind,
		can_write,
		errorHandlerSelected = $bindable(),
		error_handler_path = $bindable(),
		error_handler_args = $bindable(),
		retry = $bindable()
	}: {
		optionTabSelected: 'error_handler' | 'retries' | string
		itemKind: 'script' | 'flow'
		can_write: boolean
		errorHandlerSelected: 'custom' | 'slack' | 'teams'
		error_handler_path: string | undefined
		error_handler_args: Record<string, any>
		retry: Retry | undefined
	} = $props()
</script>

{#if ['error_handler', 'retries'].includes(optionTabSelected) && itemKind !== 'script'}
	<Alert type="info" title="Only available for scripts" class="mb-2">
		Error Handler and Retries are only available for scripts. For flows, use built-in <a
			href="https://www.windmill.dev/docs/flows/flow_error_handler"
			target="_blank">error handling</a
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
		toggleText="Alert channel on error"
		customScriptTemplate="/scripts/add?hub=hub%2F9081%2Fwindmill%2Fschedule_error_handler_template"
		customHandlerKind="script"
		bind:handlerExtraArgs={error_handler_args}
	></ErrorOrRecoveryHandler>
{:else if optionTabSelected === 'retries'}
	{@const disabled = !can_write || emptyString($enterpriseLicense)}
	<FlowRetries bind:flowModuleRetry={retry} {disabled} />
{/if}
