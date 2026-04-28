<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { Alert, Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import type { AzureMode, AzureArmResource } from '$lib/gen'
	import { AzureTriggerService } from '$lib/gen'
	import { emptyStringTrimmed } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import { RefreshCw } from 'lucide-svelte'

	interface Props {
		can_write?: boolean
		headless?: boolean
		isValid?: boolean
		azure_resource_path: string
		azure_mode: AzureMode
		scope_resource_id: string
		topic_name?: string
		subscription_name: string
		event_type_filters?: string[]
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
		event_type_filters = $bindable(),
		path = ''
	}: Props = $props()

	type Edition = 'basic' | 'namespace'
	type Delivery = 'push' | 'pull'

	const edition = $derived<Edition>(azure_mode === 'basic_push' ? 'basic' : 'namespace')
	const delivery = $derived<Delivery>(azure_mode === 'namespace_pull' ? 'pull' : 'push')

	function setEdition(next: Edition) {
		if (next === 'basic') {
			azure_mode = 'basic_push'
		} else {
			azure_mode = delivery === 'pull' ? 'namespace_pull' : 'namespace_push'
		}
	}

	function setDelivery(next: Delivery) {
		azure_mode = next === 'pull' ? 'namespace_pull' : 'namespace_push'
	}

	$effect(() => {
		if (azure_mode === 'basic_push' && topic_name !== undefined && topic_name !== '') {
			topic_name = undefined
		}
	})

	$effect(() => {
		if (emptyStringTrimmed(subscription_name) && !emptyStringTrimmed(path) && $workspaceStore) {
			const generated = `windmill-${$workspaceStore}-${path.replaceAll(/[^A-Za-z0-9-]/g, '-')}`
			subscription_name = generated.slice(0, 50)
		}
	})

	const is_namespace = $derived(edition === 'namespace')

	const has_sp = $derived(!emptyStringTrimmed(azure_resource_path))
	const has_scope = $derived(!emptyStringTrimmed(scope_resource_id))
	const has_topic = $derived(!is_namespace || !emptyStringTrimmed(topic_name))
	const config_ready = $derived(has_sp && has_scope && has_topic)

	const scopeLabel = $derived(is_namespace ? 'Namespace' : 'Topic')
	const scopeTooltip = $derived(
		is_namespace
			? 'Event Grid Namespace — loaded via the service principal. Refresh after creating one.'
			: 'Basic Event Grid topic or system topic — loaded via the service principal.'
	)

	// ARM resource loading -----------------------------------------------
	let scopeResources = $state<AzureArmResource[]>([])
	let scopeLoading = $state(false)
	let scopeError = $state<string | undefined>(undefined)

	async function loadScopeResources() {
		if (!$workspaceStore || emptyStringTrimmed(azure_resource_path)) {
			scopeResources = []
			return
		}
		scopeLoading = true
		scopeError = undefined
		try {
			const result = is_namespace
				? await AzureTriggerService.listAzureNamespaces({
						workspace: $workspaceStore,
						path: azure_resource_path
					})
				: await AzureTriggerService.listAzureBasicTopics({
						workspace: $workspaceStore,
						path: azure_resource_path
					})
			scopeResources = result
			// Clear selection if it no longer matches any entry in the new list
			// (e.g. flipped edition, swapped SP).
			if (scope_resource_id && !result.some((r) => r.id === scope_resource_id)) {
				scope_resource_id = ''
				topic_name = undefined
			}
		} catch (e: any) {
			scopeError = e?.body?.error?.message ?? e?.message ?? String(e)
			scopeResources = []
		} finally {
			scopeLoading = false
		}
	}

	$effect(() => {
		// Re-fetch when SP changes or edition flips.
		void azure_resource_path
		void edition
		loadScopeResources()
	})

	const scopeItems = $derived(
		scopeResources.map((r) => ({
			label: r.name + (r.type?.toLowerCase().includes('systemtopic') ? ' (system)' : ''),
			value: r.id,
			subtext: r.id
		}))
	)

	let topics = $state<{ name: string; id: string }[]>([])
	let topicsLoading = $state(false)
	let topicsError = $state<string | undefined>(undefined)

	async function loadTopics() {
		if (
			!is_namespace ||
			!$workspaceStore ||
			emptyStringTrimmed(azure_resource_path) ||
			emptyStringTrimmed(scope_resource_id)
		) {
			topics = []
			return
		}
		topicsLoading = true
		topicsError = undefined
		try {
			const result = await AzureTriggerService.listAzureNamespaceTopics({
				workspace: $workspaceStore,
				path: azure_resource_path,
				requestBody: { scope_resource_id }
			})
			topics = (result as any[]).map((t) => ({ name: t.name, id: t.id }))
			if (topic_name && !topics.some((t) => t.name === topic_name)) {
				topic_name = undefined
			}
		} catch (e: any) {
			topicsError = e?.body?.error?.message ?? e?.message ?? String(e)
			topics = []
		} finally {
			topicsLoading = false
		}
	}

	$effect(() => {
		void scope_resource_id
		void azure_resource_path
		void is_namespace
		loadTopics()
	})

	const topicItems = $derived(topics.map((t) => ({ label: t.name, value: t.name })))

	const AZURE_SUB_NAME_RE = /^[A-Za-z0-9-]{3,50}$/

	const subscriptionNameError = $derived.by(() => {
		if (emptyStringTrimmed(subscription_name)) return ''
		return AZURE_SUB_NAME_RE.test(subscription_name)
			? ''
			: 'Must be 3–50 chars, letters/digits/hyphens only.'
	})

	$effect(() => {
		const resource_ok = !emptyStringTrimmed(azure_resource_path)
		const scope_ok = !emptyStringTrimmed(scope_resource_id)
		const sub_ok = !emptyStringTrimmed(subscription_name) && subscriptionNameError === ''
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
			tooltip="Windmill resource of type `azure` (azureTenantId, azureClientId, azureClientSecret)."
		>
			<ResourcePicker bind:value={azure_resource_path} resourceType="azure" disabled={!can_write} />
		</Subsection>

		{#if has_sp}
			<Subsection
				label="Event Grid edition"
				tooltip="Namespace is the newer product (2024 GA) with push + pull delivery and CloudEvents 1.0. Basic is the legacy service — use it for Azure system topics (Storage / Key Vault events etc.)."
			>
				<ToggleButtonGroup
					selected={edition}
					onSelected={(v) => setEdition(v as Edition)}
					disabled={!can_write}
				>
					{#snippet children({ item })}
						<ToggleButton value="namespace" label="Namespace" {item} />
						<ToggleButton value="basic" label="Basic" {item} />
					{/snippet}
				</ToggleButtonGroup>
			</Subsection>

			{#if is_namespace}
				<Subsection
					label="Delivery mode"
					tooltip="Push: Azure POSTs events to this Windmill instance (requires a public URL). Pull: Windmill polls Azure, better for retries and not requiring inbound exposure."
				>
					<ToggleButtonGroup
						selected={delivery}
						onSelected={(v) => setDelivery(v as Delivery)}
						disabled={!can_write}
					>
						{#snippet children({ item })}
							<ToggleButton value="pull" label="Pull" {item} />
							<ToggleButton value="push" label="Push" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</Subsection>
			{/if}

			<Subsection label={scopeLabel} tooltip={scopeTooltip}>
				<div class="flex items-center gap-2">
					<div class="flex-1">
						<Select
							bind:value={scope_resource_id}
							items={scopeItems}
							placeholder={scopeLoading ? 'Loading…' : `Select ${scopeLabel.toLowerCase()}`}
							disabled={!can_write || scopeLoading}
							clearable
						/>
					</div>
					<Button
						variant="subtle"
						startIcon={{ icon: RefreshCw }}
						iconOnly
						disabled={!can_write || scopeLoading}
						onclick={loadScopeResources}
					/>
				</div>
				{#if scopeError}
					<p class="text-xs text-red-600 mt-1">{scopeError}</p>
				{:else if !scopeLoading && scopeResources.length === 0}
					<p class="text-xs text-tertiary mt-1">
						No {scopeLabel.toLowerCase()} found. Create one in Azure and click refresh.
					</p>
				{/if}
			</Subsection>
		{/if}

		{#if has_sp && is_namespace && has_scope}
			<Subsection label="Topic" tooltip="Topic inside the selected Namespace.">
				<div class="flex items-center gap-2">
					<div class="flex-1">
						<Select
							bind:value={topic_name}
							items={topicItems}
							placeholder={topicsLoading ? 'Loading…' : 'Select topic'}
							disabled={!can_write || topicsLoading}
							clearable
						/>
					</div>
					<Button
						variant="subtle"
						startIcon={{ icon: RefreshCw }}
						iconOnly
						disabled={!can_write || topicsLoading}
						onclick={loadTopics}
					/>
				</div>
				{#if topicsError}
					<p class="text-xs text-red-600 mt-1">{topicsError}</p>
				{/if}
			</Subsection>
		{/if}

		{#if config_ready}
			<Subsection
				label="Subscription name"
				tooltip="Auto-generated from the trigger path. Windmill creates this subscription on Azure (or overwrites it) on save."
			>
				<TextInput
					bind:value={subscription_name}
					inputProps={{
						placeholder: 'leave empty to auto-generate',
						disabled: !can_write
					}}
					error={subscriptionNameError}
				/>
				{#if subscriptionNameError}
					<p class="text-xs text-red-600 mt-1">{subscriptionNameError}</p>
				{/if}
				<div class="mt-2">
					<Alert title="Saving overwrites this subscription on Azure" type="warning" size="xs">
						If a subscription with this name already exists with an incompatible delivery mode
						(Push↔Pull) or endpoint type, Windmill will delete and recreate it — any in-flight
						events in its queue will be dropped.
					</Alert>
				</div>
			</Subsection>

			<Subsection
				label="Event type filters"
				tooltip="Optional. One per line (or comma-separated). Forwarded as includedEventTypes."
			>
				<TextInput
					value={filterText}
					underlyingInputEl="textarea"
					class="font-mono"
					inputProps={{
						placeholder: 'Microsoft.Storage.BlobCreated',
						disabled: !can_write,
						rows: 3,
						oninput: (e) => setFilterText(e.currentTarget.value)
					}}
				/>
			</Subsection>
		{/if}
	</div>
</Section>
