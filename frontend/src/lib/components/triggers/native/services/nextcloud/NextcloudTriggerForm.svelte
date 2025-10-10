<script lang="ts">
	import type { ServiceFormProps } from '../../utils'
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NextCloudEventType } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import Label from '$lib/components/Label.svelte'
	import { Button } from '$lib/components/common'
	import { RefreshCw, Download } from 'lucide-svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'

	let {
		config,
		errors,
		resources,
		onConfigChange,
		onTest,
		disabled = false
	}: ServiceFormProps = $props()

	let eventTypes = $state<NextCloudEventType[]>([])
	let loadingEvents = $state(false)

	// Load available event types when nextcloud resource changes
	$effect(() => {
		if (config.nextcloud_resource_path) {
			loadEventTypes()
		}
	})

	async function loadEventTypes() {
		if (!config.nextcloud_resource_path) return

		loadingEvents = true
		try {
			const events = await NativeTriggerService.listNextCloudEvents({
				workspace: $workspaceStore!,
				resourcePath: config.nextcloud_resource_path
			})
			eventTypes = events
		} catch (err: any) {
			console.error('Failed to load event types:', err)
			sendUserToast(`Failed to load event types: ${err.body || err.message}`, true)
			eventTypes = []
		} finally {
			loadingEvents = false
		}
	}

	function updateConfig(field: string, value: any) {
		onConfigChange({ ...config, [field]: value })
	}
</script>

<div class="space-y-4">
	<!-- Nextcloud Resource -->
	<div>
		<Label label="Nextcloud Resource" required />
		<select
			bind:value={config.nextcloud_resource_path}
			class="windmill-input"
			class:error={errors.nextcloud_resource_path}
			{disabled}
			onchange={(e) => updateConfig('nextcloud_resource_path', (e.target as HTMLSelectElement).value)}
		>
			<option value="">Select Nextcloud resource...</option>
			{#each resources as resource}
				<option value={resource.path}>{resource.path}</option>
			{/each}
		</select>
		{#if errors.nextcloud_resource_path}
			<div class="text-red-500 text-xs mt-1">{errors.nextcloud_resource_path}</div>
		{/if}
	</div>

	<!-- Event Type -->
	<div>
		<Label label="Event Type" required />
		<div class="flex gap-2 items-end">
			<div class="flex-1">
				{#if loadingEvents}
					<Skeleton layout={[[1]]} />
				{:else}
					<select
						bind:value={config.event_type}
						class="windmill-input"
						class:error={errors.event_type}
						{disabled}
						onchange={(e) => updateConfig('event_type', (e.target as HTMLSelectElement).value)}
					>
						<option value="">Select event type...</option>
						{#each eventTypes as eventType}
							<option value={eventType.id}>
								{eventType.name}
								{#if eventType.description}
									- {eventType.description}
								{/if}
							</option>
						{/each}
					</select>
				{/if}
			</div>
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: RefreshCw }}
				on:click={loadEventTypes}
				disabled={disabled || !config.nextcloud_resource_path || loadingEvents}
			>
				Refresh
			</Button>
		</div>
		{#if errors.event_type}
			<div class="text-red-500 text-xs mt-1">{errors.event_type}</div>
		{/if}
	</div>

	<!-- File Path Filter (Optional) -->
	<div>
		<Label label="File Path Filter" />
		<div class="text-xs text-secondary mt-1">Optional pattern to filter files (e.g., /Documents/*.pdf)</div>
		<input
			bind:value={config.file_path}
			placeholder="/path/to/files/*"
			class="windmill-input"
			{disabled}
			oninput={(e) => updateConfig('file_path', (e.target as HTMLInputElement).value)}
		/>
	</div>

	<!-- Advanced Configuration -->
	<details class="group">
		<summary class="cursor-pointer text-sm font-medium text-primary group-open:mb-3">
			Advanced Configuration
		</summary>
		<div class="space-y-3 pl-4 border-l-2 border-gray-200 dark:border-gray-700">
			<!-- Webhook URL (Read-only info) -->
			<div>
				<Label label="Webhook URL" />
			<div class="text-xs text-secondary mt-1">This URL will be registered with Nextcloud</div>
				<input
					value={`${window.location.origin}/api/native_triggers/nextcloud/webhook`}
					class="windmill-input font-mono text-xs"
					readonly
					disabled
				/>
			</div>

			<!-- Custom Config JSON -->
			<div>
				<Label label="Custom Configuration" />
			<div class="text-xs text-secondary mt-1">Additional JSON configuration for the trigger</div>
				<textarea
					bind:value={config.custom_config}
					placeholder={'{"key": "value"}'}
					class="windmill-input font-mono text-xs"
					rows="3"
					{disabled}
					oninput={(e) => {
						try {
							const value = (e.target as HTMLTextAreaElement).value
							const parsed = value ? JSON.parse(value) : undefined
							updateConfig('config', parsed)
						} catch {
							// Invalid JSON, keep as string for now
							updateConfig('custom_config', (e.target as HTMLTextAreaElement).value)
						}
					}}
				></textarea>
			</div>
		</div>
	</details>

	<!-- Test Connection -->
	{#if onTest}
		<div class="pt-4 border-t border-gray-200 dark:border-gray-700">
			<Button
				size="xs"
				color="blue"
				startIcon={{ icon: Download }}
				on:click={onTest}
				disabled={disabled || !config.nextcloud_resource_path}
			>
				Test Nextcloud Connection
			</Button>
		</div>
	{/if}
</div>