<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NativeServiceName, RunnableKind } from '$lib/gen/types.gen'
	import type { ExtendedNativeTrigger } from './utils'
	import { validateCommonFields, getServiceConfig, getTemplateUrl } from './utils'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Drawer } from '$lib/components/common'
	import { X, Save, Pipette } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import NextcloudTriggerForm from './services/nextcloud/NextcloudTriggerForm.svelte'

	interface Props {
		service: NativeServiceName
		onUpdate?: () => void
	}

	let { service, onUpdate }: Props = $props()

	const serviceConfig = $derived(getServiceConfig(service))
	const scriptTemplateUrl = $derived.by(() => {
		const templateId = getTemplateUrl(service, 'script')
		return templateId ? `/scripts/add?hub=${encodeURIComponent(templateId)}` : undefined
	})
	const flowTemplateUrl = $derived.by(() => {
		const templateId = getTemplateUrl(service, 'flow')
		return templateId ? `/flows/add?hub=${encodeURIComponent(templateId)}` : undefined
	})

	let serviceFormRef = $state<any>(null)

	const ServiceFormComponent = $derived.by(() => {
		switch (service) {
			case 'nextcloud':
				return NextcloudTriggerForm
			default:
				return null
		}
	})

	let isOpen = $state(false)
	let isNew = $state(false)
	let loading = $state(false)
	let loadingConfig = $state(false)
	let config = $state<Record<string, any>>({})
	let externalData = $state<any>(null)
	let errors = $state<Record<string, string>>({})
	let showCustomRawEditor = $state(false)
	let customRawConfig = $state('')

	let runnablePath = $state('')
	let runnableKind = $state<RunnableKind>('script')
	let resourcePath = $state('')
	let summary = $state('')
	let triggerId = $state<number | null>(null)

	export function openNew() {
		isNew = true
		config = {}
		externalData = null
		errors = {}
		runnablePath = ''
		runnableKind = 'script'
		resourcePath = ''
		summary = ''
		triggerId = null
		showCustomRawEditor = false
		customRawConfig = ''
		loadingConfig = false
		isOpen = true
	}

	export async function openEdit(trigger: ExtendedNativeTrigger) {
		isNew = false
		config = trigger.metadata || {}
		externalData = null
		errors = {}
		runnablePath = trigger.runnable_path
		runnableKind = trigger.runnable_kind === 'flow' ? 'flow' : 'script'
		resourcePath = trigger.resource_path
		summary = trigger.summary
		triggerId = trigger.id
		showCustomRawEditor = false
		customRawConfig = JSON.stringify(config, null, 2)
		loadingConfig = true
		isOpen = true

		try {
			const fullTrigger = await NativeTriggerService.getNativeTrigger({
				workspace: $workspaceStore!,
				serviceName: service,
				path: trigger.runnable_path,
				id: trigger.id
			})
			
			externalData = fullTrigger.external_data
			
			if (fullTrigger.external_data) {
				customRawConfig = JSON.stringify(fullTrigger.external_data, null, 2)
			}
		} catch (err: any) {
			const errorMessage = err.body || err.message || ''
			
			if (errorMessage.includes('no longer exists on external service') && errorMessage.includes('automatically deleted')) {
				sendUserToast(
					`Trigger was automatically deleted because it no longer exists on ${serviceConfig?.serviceDisplayName}. The editor will close.`,
					true
				)
				close()
				onUpdate?.()
			} else {
				sendUserToast(`Failed to load trigger configuration: ${errorMessage}`, true)
				externalData = null
			}
		} finally {
			loadingConfig = false
		}
	}

	function close() {
		isOpen = false
	}

	function validateForm(): boolean {
		const commonErrors = validateCommonFields({
			runnable_path: runnablePath,
			resource_path: resourcePath
		})

		let serviceErrors: Record<string, string> = {}
		if (serviceFormRef?.validate) {
			serviceErrors = serviceFormRef.validate()
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
				payload: config
			}

			// Debug: log the payload being sent
			console.log('Sending payload:', JSON.stringify(payload, null, 2))

			if (isNew) {
				await NativeTriggerService.createNativeTrigger({
					workspace: $workspaceStore!,
					serviceName: service,
					requestBody: payload
				})
				sendUserToast(`${serviceConfig?.serviceDisplayName} trigger created`)
			} else {
				if (!triggerId) {
					sendUserToast('No trigger ID available for update', true)
					return
				}

				await NativeTriggerService.updateNativeTrigger({
					workspace: $workspaceStore!,
					serviceName: service,
					id: triggerId,
					requestBody: payload
				})
				sendUserToast(`${serviceConfig?.serviceDisplayName} trigger updated`)
			}

			close()
			onUpdate?.()
		} catch (err: any) {
			sendUserToast(`Failed to save trigger: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}
	let templateUrl = $derived(runnableKind === 'script' ? scriptTemplateUrl : flowTemplateUrl)
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
				<div class="rounded-md p-4 space-y-4">
					<Section label="Runnable">
						<p class="text-xs text-tertiary">
							Pick a script or flow to be triggered <Required required={true} />
						</p>
						<div class="flex gap-2 items-end">
							<div class="flex-1">
								<ScriptPicker
									bind:scriptPath={runnablePath}
									bind:itemKind={runnableKind}
									kinds={['script']}
									allowFlow={true}
									clearable
								/>
							</div>
							<Button
								href={templateUrl}
								target="_blank"
								startIcon={{ icon: Pipette }}
								disabled={loading}
							>
								Create from {serviceConfig?.serviceDisplayName} template
							</Button>
						</div>
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

				{#if loadingConfig}
					<div class="rounded-md p-4 space-y-4">
						<h3 class="text-md font-medium text-primary">
							{serviceConfig?.serviceDisplayName} Configuration
						</h3>
						<div class="flex items-center gap-2 text-tertiary">
							<div class="animate-spin h-4 w-4 border-2 border-gray-300 border-t-gray-600 rounded-full"></div>
							Loading configuration from {serviceConfig?.serviceDisplayName}...
						</div>
					</div>
				{:else if ServiceFormComponent}
					<ServiceFormComponent
						bind:this={serviceFormRef}
						bind:config
						bind:errors
						bind:showCustomRawEditor
						bind:customRawConfig
						bind:resourcePath
						{externalData}
						disabled={loading || loadingConfig}
					/>
				{:else if resourcePath}
					<div class="rounded-md p-4 space-y-4">
						<h3 class="text-md font-medium text-primary">
							{serviceConfig?.serviceDisplayName} Configuration
						</h3>
						<div class="text-red-500 text-sm">
							Failed to load service configuration component for {service}.
						</div>
					</div>
				{:else}
					<div class="rounded-md p-4 space-y-4">
						<h3 class="text-md font-medium text-primary">
							{serviceConfig?.serviceDisplayName} Configuration
						</h3>
						<div class="text-tertiary">
							Please select a resource to load configuration options.
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>
</Drawer>
