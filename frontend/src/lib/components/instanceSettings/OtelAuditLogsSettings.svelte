<script lang="ts">
	import type { Writable } from 'svelte/store'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import OtelAuditWorkspaces from './OtelAuditWorkspaces.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()
</script>

{#if $values.otel}
	{#if $values.otel.logs_enabled}
		<Toggle
			{disabled}
			id="logs_exclude_audit"
			bind:checked={$values.otel.logs_exclude_audit}
			options={{
				right: 'Leave audit logs out of the logs export',
				rightTooltip: 'For when the audit logs export below carries them.'
			}}
		/>
	{/if}

	<div class="flex flex-col gap-4 pt-4 border-t">
		<div class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Audit logs export</span>
			<span class="text-2xs text-secondary">
				A second logs exporter that receives audit logs only, whatever the log level, for example to
				keep them in long-term storage. It has its own endpoint and headers.
			</span>
		</div>
		<Toggle
			{disabled}
			id="audit_logs_enabled"
			bind:checked={$values.otel.audit_logs_enabled}
			options={{ right: 'Export audit logs' }}
		/>
		{#if $values.otel.audit_logs_enabled}
			<div class="flex flex-col gap-1">
				<label for="OTEL_AUDIT_LOGS_ENDPOINT" class="block text-xs font-semibold text-emphasis"
					>Endpoint</label
				>
				<TextInput
					inputProps={{
						type: 'text',
						placeholder: 'https://audit-collector.example.com:4317',
						id: 'OTEL_AUDIT_LOGS_ENDPOINT',
						disabled
					}}
					bind:value={$values.otel.audit_logs_endpoint}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label for="OTEL_AUDIT_LOGS_HEADERS" class="block text-xs font-semibold text-emphasis"
					>Headers</label
				>
				<TextInput
					inputProps={{
						type: 'text',
						placeholder: 'Authorization=Bearer my-secret-token',
						id: 'OTEL_AUDIT_LOGS_HEADERS',
						disabled
					}}
					bind:value={$values.otel.audit_logs_headers}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label for="OTEL_AUDIT_LOGS_PROTOCOL" class="block text-xs font-semibold text-emphasis"
					>Protocol</label
				>
				<Select
					id="OTEL_AUDIT_LOGS_PROTOCOL"
					{disabled}
					clearable
					placeholder="grpc (default)"
					items={[{ label: 'http/protobuf', value: 'http/protobuf' }]}
					bind:value={$values.otel.audit_logs_protocol}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<span class="block text-xs font-semibold text-emphasis">Workspaces</span>
				<span class="text-2xs text-secondary">
					Export the audit logs of these workspaces only. Instance-level entries are always
					exported. Leave empty for all workspaces.
				</span>
				<OtelAuditWorkspaces bind:value={$values.otel.audit_logs_workspaces} {disabled} />
			</div>
		{/if}
	</div>
{/if}
