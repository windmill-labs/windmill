<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NextCloudEventType } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { getNextcloudSchema } from '../../utils'

	interface Props {
		serviceConfig: Record<string, any>
		errors: Record<string, string>
		disabled?: boolean
		externalData?: any
		path?: string
		isFlow?: boolean
		hash?: string
		token?: string
		triggerTokens?: any
		scopes?: string[]
		loading: boolean
	}

	let {
		serviceConfig = $bindable(),
		errors = $bindable(),
		disabled = false,
		externalData = undefined,
		token = $bindable(''),
		triggerTokens = $bindable(undefined),
		loading = $bindable()
	}: Props = $props()

	let availableEvents = $state<NextCloudEventType[]>([])
	let serviceSchema = $state<any>(null)

	async function loadAvailableEvents() {
		if (!$workspaceStore) {
			availableEvents = []
			return
		}

		loading = true
		try {
			const events = await NativeTriggerService.listNextCloudEvents({
				workspace: $workspaceStore!
			})
			availableEvents = events
			serviceSchema = getNextcloudSchema(events)
		} catch (err: any) {
			console.error('Failed to load NextCloud events:', err)
			sendUserToast(`Failed to load available events: ${err.body || err.message}`, true)
			availableEvents = []
		} finally {
			loading = false
		}
	}

	export function validate(): Record<string, string> {
		let serviceErrors: Record<string, string> = {}

		if (serviceSchema && serviceSchema.required) {
			for (const requiredField of serviceSchema.required) {
				if (!serviceConfig[requiredField]) {
					const fieldTitle = serviceSchema.properties?.[requiredField]?.title || requiredField
					serviceErrors[requiredField] = `${fieldTitle} is required`
				}
			}
		}

		if (!serviceConfig.event && availableEvents.length === 0 && !loading) {
			serviceErrors.event =
				'Unable to load available events. Please check your resource configuration.'
		}

		return serviceErrors
	}

	let externalDataApplied = $state(false)

	$effect(() => {
		if ($workspaceStore) {
			loadAvailableEvents()
		}
	})

	$effect(() => {
		if (externalData && !externalDataApplied) {
			serviceConfig = { ...serviceConfig, ...externalData }
			externalDataApplied = true
		}
	})

	$effect(() => {
		if (!externalData) {
			externalDataApplied = false
		}
	})
</script>

<Section label="Nextcloud configuration">
	{#if loading}
		<div class="flex items-center gap-2 text-secondary text-xs">
			<Loader2 class="animate-spin" size={16} />
			Loading available events...
		</div>
	{:else if availableEvents.length === 0}
		<div class="text-red-500 text-xs space-y-2">
			<div
				>No events available. Please ensure your workspace has a connected Nextcloud integration.</div
			>
			<div class="flex gap-2">
				<Button variant="default" on:click={loadAvailableEvents} {disabled}>
					Retry loading events
				</Button>
				<Button variant="subtle" href="/workspace_settings?tab=integrations" target="_blank">
					Manage integrations
				</Button>
			</div>
		</div>
	{:else if serviceSchema}
		<SchemaForm
			schema={serviceSchema}
			bind:args={serviceConfig}
			isValid={true}
			compact={true}
			prettifyHeader={true}
			{disabled}
		/>
	{:else}
		<div class="text-secondary text-xs">
			Please ensure Nextcloud workspace integration is connected to load available configuration
			options.
		</div>
	{/if}
</Section>
