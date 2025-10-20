<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NextCloudEventType } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import CodeEditor from '$lib/components/SimpleEditor.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	interface Props {
		config: Record<string, any>
		errors: Record<string, string>
		disabled?: boolean
		showCustomRawEditor?: boolean
		customRawConfig?: string
		externalData?: any
		path?: string
		isFlow?: boolean
		hash?: string
		token?: string
		triggerTokens?: any
		scopes?: string[]
	}

	let {
		config = $bindable(),
		errors = $bindable(),
		disabled = false,
		showCustomRawEditor = $bindable(false),
		customRawConfig = $bindable(''),
		externalData = undefined,
		token = $bindable(''),
		triggerTokens = $bindable(undefined)
	}: Props = $props()

	let availableEvents = $state<NextCloudEventType[]>([])
	let loadingEvents = $state(false)
	let serviceSchema = $state<any>(null)

	function getNextcloudSchema() {
		return {
			type: 'object',
			properties: {
				event: {
					type: 'string',
					title: 'Event',
					description: 'The type of Nextcloud event to listen for',
					enum: availableEvents.map((e) => e.path),
					enumLabels: availableEvents.reduce(
						(acc, cur) => ({ ...acc, [cur.path]: cur.description }),
						{}
					)
				},
				event_filter: {
					type: 'object',
					title: 'Event Filter',
					description: 'Optional filter criteria for the event (JSON object)'
				},
				user_id_filter: {
					type: 'string',
					title: 'User ID Filter',
					description: 'Filter events by specific user ID'
				},
				headers: {
					type: 'object',
					title: 'Headers',
					description: 'Optional HTTP headers to include (JSON object)'
				}
			},
			required: ['event']
		}
	}

	async function loadAvailableEvents() {
		if (!$workspaceStore) {
			availableEvents = []
			return
		}

		loadingEvents = true
		try {
			const events = await NativeTriggerService.listNextCloudEvents({
				workspace: $workspaceStore!
			})
			availableEvents = events
			serviceSchema = getNextcloudSchema()
		} catch (err: any) {
			console.error('Failed to load NextCloud events:', err)
			sendUserToast(`Failed to load available events: ${err.body || err.message}`, true)
			availableEvents = []
		} finally {
			loadingEvents = false
		}
	}

	export function validate(): Record<string, string> {
		let serviceErrors: Record<string, string> = {}

		if (serviceSchema && serviceSchema.required) {
			for (const requiredField of serviceSchema.required) {
				if (!config[requiredField]) {
					const fieldTitle = serviceSchema.properties?.[requiredField]?.title || requiredField
					serviceErrors[requiredField] = `${fieldTitle} is required`
				}
			}
		}

		if (!config.event && availableEvents.length === 0 && !loadingEvents) {
			serviceErrors.event =
				'Unable to load available events. Please check your resource configuration.'
		}

		return serviceErrors
	}

	export function reset() {
		availableEvents = []
		loadingEvents = false
		serviceSchema = getNextcloudSchema()
	}

	function parseCustomRawConfig() {
		try {
			const parsed = JSON.parse(customRawConfig)
			config = parsed
			errors = {}
			sendUserToast('Configuration loaded from JSON')
		} catch (err) {
			sendUserToast('Invalid JSON format', true)
		}
	}

	function updateCustomRawConfig() {
		if (showCustomRawEditor) {
			customRawConfig = JSON.stringify(config, null, 2)
		}
	}

	let externalDataApplied = $state(false)

	$effect(() => {
		updateCustomRawConfig()
	})

	$effect(() => {
		if ($workspaceStore) {
			loadAvailableEvents()
		}
	})

	$effect(() => {
		if (externalData && !externalDataApplied) {
			config = { ...config, ...externalData }
			externalDataApplied = true
		}
	})

	$effect(() => {
		if (!externalData) {
			externalDataApplied = false
		}
	})
</script>

<div class="rounded-md p-4 space-y-4">
	<div class="flex items-center justify-between">
		<h3 class="text-md font-medium text-primary">Nextcloud Configuration</h3>
		<Toggle
			bind:checked={showCustomRawEditor}
			options={{ right: 'JSON Editor' }}
			size="xs"
			{disabled}
		/>
	</div>

	{#if showCustomRawEditor}
		<div class="space-y-2">
			<Label label="Custom Configuration (JSON)" />
			<CodeEditor
				bind:code={customRawConfig}
				lang="json"
				fixedOverflowWidgets={false}
				class="h-64"
			/>
			<Button size="xs" color="blue" on:click={parseCustomRawConfig} {disabled}>Parse JSON</Button>
		</div>
	{:else if loadingEvents}
		<div class="flex items-center gap-2 text-tertiary">
			<div class="animate-spin w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full"
			></div>
			Loading available events...
		</div>
	{:else if availableEvents.length === 0}
		<div class="text-red-500 text-sm space-y-2">
			<div
				>No events available. Please ensure your workspace has a connected Nextcloud integration.</div
			>
			<div class="flex gap-2">
				<Button size="xs" color="blue" on:click={loadAvailableEvents} {disabled}>
					Retry Loading Events
				</Button>
				<Button
					size="xs"
					color="light"
					variant="border"
					href="/workspace_settings?tab=integrations"
					target="_blank"
				>
					Manage Integrations
				</Button>
			</div>
		</div>
	{:else if serviceSchema}
		<SchemaForm
			schema={serviceSchema}
			bind:args={config}
			isValid={true}
			compact={true}
			prettifyHeader={true}
			{disabled}
		/>
	{:else}
		<div class="text-tertiary">
			Please ensure NextCloud workspace integration is connected to load available configuration
			options.
		</div>
	{/if}
</div>
