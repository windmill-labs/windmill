<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NativeServiceName, RunnableKind, NextCloudEventType } from '$lib/gen/types.gen'
	import type { ExtendedNativeTrigger } from './utils'
	import { validateCommonFields, getServiceConfig } from './utils'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Drawer } from '$lib/components/common'
	import { X, Save } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import CodeEditor from '$lib/components/SimpleEditor.svelte'
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	interface Props {
		service: NativeServiceName
		onUpdate?: () => void
	}

	let { service, onUpdate }: Props = $props()

	const serviceConfig = $derived(getServiceConfig(service))

	// Get schema for the service - dynamic schema based on available events
	function getServiceSchema(serviceName: NativeServiceName) {
		switch (serviceName) {
			case 'nextcloud':
				return {
					type: 'object',
					properties: {
						event: {
							type: 'string',
							title: 'Event',
							description: 'The type of Nextcloud event to listen for',
							enum: availableEvents.map((e) => e.name),
							enumNames: availableEvents.map((e) => e.description || e.name)
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
			default:
				return null
		}
	}

	// Load available events from the NextCloud instance
	async function loadAvailableEvents() {
		if (!resourcePath || !$workspaceStore) {
			availableEvents = []
			return
		}

		loadingEvents = true
		try {
			const events = await NativeTriggerService.listNextCloudEvents({
				workspace: $workspaceStore,
				resourcePath: resourcePath
			})
			availableEvents = events
			// Update schema after events are loaded
			serviceSchema = getServiceSchema(service)
		} catch (err: any) {
			console.error('Failed to load NextCloud events:', err)
			sendUserToast(`Failed to load available events: ${err.body || err.message}`, true)
			availableEvents = []
		} finally {
			loadingEvents = false
		}
	}

	// State
	let isOpen = $state(false)
	let isNew = $state(false)
	let loading = $state(false)
	let config = $state<Record<string, any>>({})
	let errors = $state<Record<string, string>>({})
	let showCustomRawEditor = $state(false)
	let customRawConfig = $state('')
	let serviceSchema = $state<any>(null)
	let availableEvents = $state<NextCloudEventType[]>([])
	let loadingEvents = $state(false)

	// Form fields
	let runnablePath = $state('')
	let runnableKind = $state<RunnableKind>('script')
	let resourcePath = $state('')
	let summary = $state('')

	export function openNew() {
		isNew = true
		config = {}
		errors = {}
		runnablePath = ''
		runnableKind = 'script'
		resourcePath = ''
		summary = ''
		showCustomRawEditor = false
		customRawConfig = ''
		availableEvents = []
		loadingEvents = false
		serviceSchema = getServiceSchema(service)
		isOpen = true
	}

	export function openEdit(trigger: ExtendedNativeTrigger) {
		isNew = false
		config = trigger.metadata || {}
		errors = {}
		runnablePath = trigger.runnable_path
		runnableKind = trigger.runnable_kind === 'flow' ? 'flow' : 'script'
		resourcePath = trigger.resource_path
		summary = trigger.summary
		showCustomRawEditor = false
		customRawConfig = JSON.stringify(config, null, 2)
		availableEvents = []
		loadingEvents = false
		serviceSchema = getServiceSchema(service)
		isOpen = true
	}

	function close() {
		isOpen = false
	}

	function validateForm(): boolean {
		const commonErrors = validateCommonFields({
			runnable_path: runnablePath,
			resource_path: resourcePath
		})

		// Schema-based validation for service-specific fields
		let serviceErrors: Record<string, string> = {}
		if (serviceSchema && serviceSchema.required) {
			for (const requiredField of serviceSchema.required) {
				if (!config[requiredField]) {
					const fieldTitle = serviceSchema.properties?.[requiredField]?.title || requiredField
					serviceErrors[requiredField] = `${fieldTitle} is required`
				}
			}
		}

		// Add specific validation for event field if events haven't loaded yet
		if (
			service === 'nextcloud' &&
			!config.event &&
			availableEvents.length === 0 &&
			!loadingEvents
		) {
			serviceErrors.event =
				'Unable to load available events. Please check your resource configuration.'
		}

		errors = { ...commonErrors, ...serviceErrors }
		return Object.keys(errors).length === 0
	}

	async function save() {
		if (!validateForm()) {
			sendUserToast('Please fix validation errors', true)
			return
		}

		loading = true
		try {
			const payload = {
				external_id: '',
				runnable_path: runnablePath,
				runnable_kind: runnableKind,
				resource_path: resourcePath,
				summary: summary || undefined,
				...config
			}

			if (isNew) {
				await NativeTriggerService.createNativeTrigger({
					workspace: $workspaceStore!,
					serviceName: service,
					requestBody: payload
				})
				sendUserToast(`${serviceConfig?.serviceDisplayName} trigger created`)
			} else {
				// TODO: Update functionality needs trigger ID from parent component
				sendUserToast('Update functionality not available in this view')
				return
			}

			close()
			onUpdate?.()
		} catch (err: any) {
			sendUserToast(`Failed to save trigger: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	function updateCustomRawConfig() {
		if (showCustomRawEditor) {
			customRawConfig = JSON.stringify(config, null, 2)
		}
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

	$effect(() => {
		updateCustomRawConfig()
	})

	$effect(() => {
		if (resourcePath && $workspaceStore) {
			loadAvailableEvents()
		}
	})
</script>

<Drawer bind:open={isOpen} size="900px">
	<div class="flex flex-col h-full">
		<div
			class="flex justify-between items-center p-4 border-b border-gray-200 dark:border-gray-700"
		>
			<h2 class="text-lg font-semibold">
				{isNew ? 'Create' : 'Edit'}
				{serviceConfig?.serviceDisplayName} Trigger
			</h2>

			<div class="flex gap-2">
				<Button
					size="xs"
					color="blue"
					startIcon={{ icon: Save }}
					on:click={save}
					disabled={loading}
				>
					{isNew ? 'Create' : 'Update'} Trigger
				</Button>
				<Button size="xs" color="ghost" on:click={close}>
					<X size={16} />
				</Button>
			</div>
		</div>

		<div class="flex-1 overflow-y-auto p-4">
			<div class="space-y-4">
				<div class="bg-surface-secondary rounded-md p-4 space-y-4">
					<h3 class="text-md font-medium text-primary">Common Configuration</h3>

					<Section label="Runnable">
						<p class="text-xs text-tertiary">
							Pick a script or flow to be triggered <Required required={true} />
						</p>
						<ScriptPicker
							bind:scriptPath={runnablePath}
							bind:itemKind={runnableKind}
							kinds={['script']}
							allowFlow={true}
							clearable
						/>
						{#if errors.runnable_path}
							<div class="text-red-500 text-xs mt-1">{errors.runnable_path}</div>
						{/if}
					</Section>

					<div>
						<Label label="Resource Path" required />
						<ResourcePicker bind:value={resourcePath} resourceType={serviceConfig?.resourceType} />
						{#if errors.resource_path}
							<div class="text-red-500 text-xs mt-1">{errors.resource_path}</div>
						{/if}
					</div>

					<div>
						<Label label="Summary" />
						<input
							bind:value={summary}
							placeholder="Brief description of this trigger"
							class="windmill-input"
						/>
					</div>
				</div>

				<!-- Service Configuration -->
				{#if serviceSchema}
					<div class="bg-surface-secondary rounded-md p-4 space-y-4">
						<div class="flex items-center justify-between">
							<h3 class="text-md font-medium text-primary">
								{serviceConfig?.serviceDisplayName} Configuration
							</h3>
							<Toggle
								bind:checked={showCustomRawEditor}
								options={{ right: 'JSON Editor' }}
								size="xs"
							/>
						</div>

						{#if showCustomRawEditor}
							<!-- Custom JSON Editor -->
							<div class="space-y-2">
								<Label label="Custom Configuration (JSON)" />
								<CodeEditor
									bind:code={customRawConfig}
									lang="json"
									fixedOverflowWidgets={false}
									class="h-64"
								/>
								<Button size="xs" color="blue" on:click={parseCustomRawConfig}>Parse JSON</Button>
							</div>
						{:else}
							<!-- Schema Form -->
							{#if loadingEvents}
								<div class="flex items-center gap-2 text-tertiary">
									<div
										class="animate-spin w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full"
									></div>
									Loading available events...
								</div>
							{:else if availableEvents.length === 0 && resourcePath}
								<div class="text-red-500 text-sm space-y-2">
									<div>No events available. Please check your NextCloud resource configuration.</div
									>
									<Button size="xs" color="blue" on:click={loadAvailableEvents}>
										Retry Loading Events
									</Button>
								</div>
							{:else if serviceSchema}
								<SchemaForm
									schema={serviceSchema}
									bind:args={config}
									isValid={true}
									compact={true}
									prettifyHeader={true}
								/>
							{:else}
								<div class="text-tertiary">
									Please select a resource to load available configuration options.
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</div>
</Drawer>
