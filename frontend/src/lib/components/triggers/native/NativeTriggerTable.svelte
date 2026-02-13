<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import type { ExtendedNativeTrigger } from './utils'
	import { getServiceConfig } from './utils'
	import { sendUserToast } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { Eye, Pen, RefreshCw, Trash } from 'lucide-svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { goto } from '$lib/navigation'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import GoogleDriveIcon from '$lib/components/icons/GoogleDriveIcon.svelte'
	import GoogleCalendarIcon from '$lib/components/icons/GoogleCalendarIcon.svelte'

	type TriggerW = ExtendedNativeTrigger & { marked?: any }

	interface Props {
		service: NativeServiceName
		triggers: TriggerW[]
		loading?: boolean
		onEdit?: (trigger: TriggerW) => void
		onRecreate?: (trigger: TriggerW) => void
		onSync?: () => Promise<void>
	}

	let { service, triggers = [], loading = false, onEdit, onRecreate }: Props = $props()

	const serviceConfig = $derived(getServiceConfig(service))
	let deleteConfirmationOpen = $state(false)
	let triggerToDelete = $state<ExtendedNativeTrigger | null>(null)
	let isDeleting = $state(false)

	function openDeleteConfirmation(trigger: ExtendedNativeTrigger) {
		triggerToDelete = trigger
		deleteConfirmationOpen = true
	}

	function closeDeleteConfirmation() {
		deleteConfirmationOpen = false
		triggerToDelete = null
		isDeleting = false
	}

	async function confirmDeleteTrigger() {
		if (!triggerToDelete) return

		isDeleting = true
		try {
			await NativeTriggerService.deleteNativeTrigger({
				workspace: $workspaceStore!,
				serviceName: service,
				externalId: triggerToDelete.external_id
			})
			sendUserToast(`${serviceConfig?.serviceDisplayName} trigger deleted`)
			triggers = triggers.filter(
				(native_trigger) => native_trigger.external_id !== triggerToDelete?.external_id
			)
			closeDeleteConfirmation()
		} catch (err: any) {
			sendUserToast(`Failed to delete trigger: ${err.body || err.message}`, true)
			isDeleting = false
		}
	}
</script>

<div class="w-full">
	{#if loading}
		{#each new Array(6) as _}
			<Skeleton layout={[[6], 0.4]} />
		{/each}
	{:else if !triggers?.length}
		<div class="text-center text-sm font-semibold text-emphasis mt-2">
			No {serviceConfig?.serviceDisplayName} triggers
		</div>
	{:else}
		<div class="border rounded-md divide-y">
			{#each triggers as trigger (trigger.external_id)}
				{@const isFlow = trigger.is_flow}
				{@const href = `${isFlow ? '/flows/get' : '/scripts/get'}/${trigger.script_path}`}
				<div
					class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
				>
					<div class="w-full flex gap-5 items-center">
						<RowIcon kind={isFlow ? 'flow' : 'script'} />

						<a
							href="#{trigger.external_id}"
							onclick={() => onEdit?.(trigger)}
							class="min-w-0 grow hover:underline decoration-gray-400"
						>
							<div class="text-emphasis flex-wrap text-left text-xs font-semibold mb-1 truncate">
								{#if trigger.marked}
									<span class="text-xs">
										{@html trigger.marked}
									</span>
								{:else}
									{trigger.script_path}
								{/if}
							</div>
							{#if service === 'google'}
								{@const triggerType = trigger.service_config?.triggerType}
								{@const resourceName = trigger.service_config?.resourceName}
								{@const calendarName = trigger.service_config?.calendarName}
								<div class="text-secondary text-xs truncate text-left font-normal flex items-center gap-1 mb-0.5">
									{#if triggerType === 'calendar'}
										<GoogleCalendarIcon height="12px" width="12px" />
										Calendar: {calendarName || trigger.service_config?.calendarId || ''}
									{:else}
										<GoogleDriveIcon height="12px" width="12px" />
										Drive: {resourceName ? resourceName : trigger.service_config?.resourceId ? trigger.service_config.resourceId : 'All changes'}
									{/if}
								</div>
							{/if}
							<div class="text-secondary text-xs truncate text-left font-normal">
								external ID: {trigger.external_id}
							</div>
						</a>

						<div class="flex gap-2 items-center justify-end">
							<Button
								on:click={() => onEdit?.(trigger)}
								unifiedSize="md"
								startIcon={{ icon: Pen }}
								variant="subtle"
							>
								Edit
							</Button>
							<Dropdown
								size="md"
								items={[
									{
										displayName: `View ${isFlow ? 'Flow' : 'Script'}`,
										icon: Eye,
										action: () => {
											goto(href)
										}
									},
									{
										displayName: 'Delete',
										type: 'delete' as const,
										icon: Trash,
										action: () => openDeleteConfirmation(trigger)
									}
								]}
							/>
						</div>
					</div>
					{#if trigger.error}
						<div class="w-full flex justify-between items-center gap-2">
							<div class="text-2xs font-normal text-red-500 truncate">
								Error: {trigger.error}
							</div>
							<Button
								size="xs"
								variant="subtle"
								startIcon={{ icon: RefreshCw }}
								on:click={() => onRecreate?.(trigger)}
							>
								Recreate
							</Button>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<ConfirmationModal
	open={deleteConfirmationOpen}
	title="Delete {serviceConfig?.serviceDisplayName} trigger"
	confirmationText="Delete"
	loading={isDeleting}
	onConfirmed={confirmDeleteTrigger}
	onCanceled={closeDeleteConfirmation}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to delete this trigger?</span>
		{#if triggerToDelete?.external_id}
			<span>External ID: {triggerToDelete.external_id}</span>
		{/if}
		<Alert type="warning" title="Warning">
			This will permanently delete the trigger from both Windmill and {serviceConfig?.serviceDisplayName}.
			This action cannot be undone.
		</Alert>
	</div>
</ConfirmationModal>
