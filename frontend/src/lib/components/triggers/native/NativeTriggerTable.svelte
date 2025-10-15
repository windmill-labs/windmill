<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import type { ExtendedNativeTrigger } from './utils'
	import { getServiceConfig } from './utils'
	import { displayDate, sendUserToast } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { Eye, Pen, Trash, Download } from 'lucide-svelte'
	import { base } from '$app/paths'
	import { NextcloudIcon } from '$lib/components/icons'

	interface Props {
		service: NativeServiceName
		triggers: ExtendedNativeTrigger[]
		loading?: boolean
		onEdit?: (trigger: ExtendedNativeTrigger) => void
		onSync?: () => Promise<void>
		shareModal?: any
	}

	let { service, triggers = [], loading = false, onEdit }: Props = $props()

	const serviceConfig = $derived(getServiceConfig(service))

	async function deleteTrigger(trigger: ExtendedNativeTrigger) {
		try {
			await NativeTriggerService.deleteNativeTrigger({
				workspace: $workspaceStore!,
				serviceName: service,
				requestBody: {
					id: trigger.id,
					runnable_path: trigger.runnable_path
				}
			})
			sendUserToast(`${serviceConfig?.serviceDisplayName} trigger deleted`)
			triggers = triggers.filter((native_trigger) => native_trigger.id !== trigger.id)
		} catch (err: any) {
			sendUserToast(`Failed to delete trigger: ${err.body || err.message}`, true)
		}
	}

	async function fetchConfig(trigger: ExtendedNativeTrigger) {
		try {
			const fullTrigger = await NativeTriggerService.getNativeTrigger({
				workspace: $workspaceStore!,
				serviceName: service,
				path: trigger.runnable_path,
				id: trigger.id
			})
			sendUserToast('Configuration fetched from external service')
			return fullTrigger
		} catch (err: any) {
			const errorMessage = err.body || err.message || ''

			if (
				errorMessage.includes('no longer exists on external service') &&
				errorMessage.includes('automatically deleted')
			) {
				sendUserToast(
					`Trigger was automatically deleted because it no longer exists on ${serviceConfig?.serviceDisplayName}. The page will refresh.`,
					true
				)
			} else {
				sendUserToast(`Failed to fetch config: ${errorMessage}`, true)
			}
		}
	}
</script>

<div class="w-full">
	{#if loading}
		{#each new Array(6) as _}
			<Skeleton layout={[[6], 0.4]} />
		{/each}
	{:else if !triggers?.length}
		<div class="text-center text-sm text-tertiary mt-2">
			No {serviceConfig?.serviceDisplayName} triggers found
		</div>
	{:else}
		<div class="border rounded-md divide-y">
			{#each triggers as trigger}
				{@const isFlow = trigger.runnable_path.includes('/flows/')}
				{@const href = `${isFlow ? '/flows/get' : '/scripts/get'}/${trigger.runnable_path}`}
				<div
					class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
				>
					<div class="w-full flex gap-5 items-center">
						{#if service === 'nextcloud'}
							<NextcloudIcon size={24} />
						{:else}
							<RowIcon kind={isFlow ? 'flow' : 'script'} />
						{/if}

						<a
							href="#{trigger.external_id}"
							onclick={() => onEdit?.(trigger)}
							class="min-w-0 grow hover:underline decoration-gray-400"
						>
							<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
								{trigger.summary || trigger.external_id}
							</div>
							<div class="text-secondary text-xs truncate text-left font-light">
								external_id: {trigger.external_id}
							</div>
							<div class="text-tertiary text-2xs truncate text-left font-light">
								runnable: {trigger.runnable_path}
							</div>
						</a>

						<div class="flex gap-2 items-center justify-end">
							<Button {href} size="xs" startIcon={{ icon: Eye }} color="light" variant="border">
								View {isFlow ? 'Flow' : 'Script'}
							</Button>
							<Button
								size="xs"
								startIcon={{ icon: Pen }}
								color="dark"
								on:click={() => onEdit?.(trigger)}
							>
								Edit
							</Button>
							<Dropdown
								items={[
									...(serviceConfig?.supportsFetchConfig
										? [
												{
													displayName: 'Fetch Config',
													icon: Download,
													action: () => fetchConfig(trigger)
												}
											]
										: []),
									{
										displayName: 'Delete',
										type: 'delete' as const,
										icon: Trash,
										action: () => deleteTrigger(trigger)
									},
									{
										displayName: 'Audit logs',
										icon: Eye,
										href: `${base}/audit_logs?resource=${trigger.external_id}`
									}
								]}
							/>
						</div>
					</div>

					<div class="text-2xs text-tertiary text-left w-full flex justify-between items-center">
						<span>edited by {trigger.edited_by} • {displayDate(trigger.edited_at)}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
