<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { base } from '$lib/base'
	import { Url } from '$lib/components/common'
	import type { AzureMode, AzureSubscriptionMode, AzureDeliveryConfig } from '$lib/gen'
	import { emptyStringTrimmed } from '$lib/utils'

	interface Props {
		can_write?: boolean
		headless?: boolean
		isValid?: boolean
		azure_resource_path: string
		azure_mode: AzureMode
		scope_resource_id: string
		topic_name?: string
		subscription_name: string
		subscription_mode: AzureSubscriptionMode
		delivery_config?: AzureDeliveryConfig
		event_type_filters?: string[]
		max_events?: number
		max_wait_time_sec?: number
		auto_acknowledge_msg: boolean
		path?: string
	}

	let {
		can_write = false,
		headless = false,
		isValid = $bindable(false),
		azure_resource_path = $bindable(),
		azure_mode = $bindable(),
		scope_resource_id = $bindable(),
		topic_name = $bindable(),
		subscription_name = $bindable(),
		subscription_mode = $bindable(),
		delivery_config = $bindable(),
		event_type_filters = $bindable(),
		max_events = $bindable(),
		max_wait_time_sec = $bindable(),
		auto_acknowledge_msg = $bindable(),
		path = ''
	}: Props = $props()

	const DEFAULT_DELIVERY_CONFIG: AzureDeliveryConfig = { authenticate: false }

	// Ensure delivery_config exists for push modes so validation passes.
	$effect(() => {
		if (
			(azure_mode === 'basic_push' || azure_mode === 'namespace_push') &&
			delivery_config === undefined
		) {
			delivery_config = { ...DEFAULT_DELIVERY_CONFIG }
		}
		if (azure_mode === 'namespace_pull' && delivery_config !== undefined) {
			delivery_config = undefined
		}
	})

	// Clear topic_name for basic_push mode.
	$effect(() => {
		if (azure_mode === 'basic_push' && topic_name !== undefined && topic_name !== '') {
			topic_name = undefined
		}
	})

	const is_namespace = $derived(azure_mode === 'namespace_push' || azure_mode === 'namespace_pull')
	const is_push = $derived(azure_mode === 'basic_push' || azure_mode === 'namespace_push')

	$effect(() => {
		const resource_ok = !emptyStringTrimmed(azure_resource_path)
		const scope_ok = !emptyStringTrimmed(scope_resource_id)
		const sub_ok = !emptyStringTrimmed(subscription_name)
		const topic_ok = !is_namespace || !emptyStringTrimmed(topic_name)
		isValid = resource_ok && scope_ok && sub_ok && topic_ok
	})

	const filterText = $derived(event_type_filters?.join('\n') ?? '')
	function setFilterText(value: string) {
		const list = value
			.split(/\n|,/)
			.map((v) => v.trim())
			.filter((v) => v.length > 0)
		event_type_filters = list.length > 0 ? list : undefined
	}
</script>

<Section label="Azure Event Grid" {headless}>
	<div class="flex flex-col gap-6">
		<Subsection
			label="Service Principal"
			tooltip="Windmill resource containing tenant_id, client_id, client_secret, and a default subscription_id."
		>
			<ResourcePicker
				bind:value={azure_resource_path}
				resourceType="azure_service_principal"
				disabled={!can_write}
			/>
		</Subsection>

		<Subsection label="Mode" tooltip="Event Grid flavor + delivery style.">
			<ToggleButtonGroup bind:selected={azure_mode} disabled={!can_write}>
				{#snippet children({ item })}
					<ToggleButton value="basic_push" label="Basic (push)" {item} />
					<ToggleButton value="namespace_push" label="Namespace (push)" {item} />
					<ToggleButton value="namespace_pull" label="Namespace (pull)" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</Subsection>

		<Subsection
			label={is_namespace ? 'Namespace ARM resource ID' : 'Topic ARM resource ID'}
			tooltip={is_namespace
				? 'ARM id of your Event Grid Namespace, e.g. /subscriptions/.../namespaces/my-ns'
				: 'ARM id of your Event Grid topic / system topic / domain'}
		>
			<input
				type="text"
				bind:value={scope_resource_id}
				placeholder="/subscriptions/.../providers/Microsoft.EventGrid/..."
				disabled={!can_write}
				class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary font-mono"
			/>
		</Subsection>

		{#if is_namespace}
			<Subsection label="Topic name" tooltip="Topic name inside the Namespace.">
				<input
					type="text"
					bind:value={topic_name}
					disabled={!can_write}
					placeholder="my-topic"
					class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary"
				/>
			</Subsection>
		{/if}

		<Subsection label="Subscription">
			<div class="flex flex-col gap-2">
				<ToggleButtonGroup bind:selected={subscription_mode} disabled={!can_write}>
					{#snippet children({ item })}
						<ToggleButton value="create_update" label="Create / update" {item} />
						<ToggleButton value="existing" label="Use existing" {item} />
					{/snippet}
				</ToggleButtonGroup>
				<input
					type="text"
					bind:value={subscription_name}
					placeholder="subscription-name"
					disabled={!can_write}
					class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary"
				/>
			</div>
		</Subsection>

		<Subsection
			label="Event type filters"
			tooltip="Optional. One per line (or comma-separated). Forwarded as includedEventTypes."
		>
			<textarea
				value={filterText}
				oninput={(e) => setFilterText(e.currentTarget.value)}
				disabled={!can_write}
				rows="3"
				class="w-full px-3 py-2 text-sm border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary font-mono"
				placeholder="Microsoft.Storage.BlobCreated"
			></textarea>
		</Subsection>

		{#if is_push && delivery_config}
			<Subsection
				label="Push webhook URL"
				tooltip="Azure will be configured to POST events to this URL."
			>
				<Url url={`${window.location.origin}${base}/api/azure/w/\${workspace}/${path}`} />
				<Toggle
					bind:checked={delivery_config.authenticate}
					disabled={!can_write}
					options={{ right: 'Validate inbound JWT tokens' }}
				/>
				{#if delivery_config.authenticate}
					<input
						type="text"
						bind:value={delivery_config.audience}
						placeholder="expected audience"
						disabled={!can_write}
						class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary"
					/>
				{/if}
			</Subsection>
		{/if}

		{#if azure_mode === 'namespace_pull'}
			<Subsection label="Pull options">
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label class="text-xs">Max events per batch</label>
						<input
							type="number"
							bind:value={max_events}
							min="1"
							max="100"
							placeholder="10"
							disabled={!can_write}
							class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary"
						/>
					</div>
					<div>
						<label class="text-xs">Max wait time (sec)</label>
						<input
							type="number"
							bind:value={max_wait_time_sec}
							min="10"
							max="120"
							placeholder="60"
							disabled={!can_write}
							class="w-full px-3 py-2 text-sm border rounded-md bg-surface text-primary"
						/>
					</div>
				</div>
				<Toggle
					bind:checked={auto_acknowledge_msg}
					disabled={!can_write}
					options={{ right: 'Auto-acknowledge messages' }}
				/>
			</Subsection>
		{/if}
	</div>
</Section>
