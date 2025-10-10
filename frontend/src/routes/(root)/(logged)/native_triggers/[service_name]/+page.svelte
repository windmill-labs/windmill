<script lang="ts">
	import { run } from 'svelte/legacy'
	import { NativeTriggerService } from '$lib/gen'
	import type { NativeServiceName, NativeTrigger } from '$lib/gen'
	import type { ExtendedNativeTrigger } from '$lib/components/triggers/native/utils'
	import {
		isServiceAvailable,
		formatTriggerDisplayName,
		getServiceConfig
	} from '$lib/components/triggers/native/utils'
	import NativeTriggerTable from '$lib/components/triggers/native/NativeTriggerTable.svelte'
	import NativeTriggerEditor from '$lib/components/triggers/native/NativeTriggerEditor.svelte'
	import { sendUserToast, removeTriggerKindIfUnused } from '$lib/utils'
	import { userStore, workspaceStore, userWorkspaces, usedTriggerKinds } from '$lib/stores'
	import { onMount } from 'svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Button, Alert } from '$lib/components/common'
	import { Plus, AlertTriangle, RefreshCw } from 'lucide-svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { page } from '$app/stores'

	const serviceName = $derived($page.params.service_name as NativeServiceName)
	const serviceConfig = $derived(getServiceConfig(serviceName))

	let triggers: NativeTrigger[] = $state([])
	let loading = $state(true)
	let serviceAvailable = $state(false)
	let serviceSupported = $state(true)
	let editor: NativeTriggerEditor
	let shareModal: ShareModal | undefined = $state()

	let filteredItems: ExtendedNativeTrigger[] = $state([])
	let filter = $state('')

	async function checkServiceAvailability() {
		if (!serviceConfig) {
			serviceSupported = false
			return
		}

		if ($workspaceStore) {
			serviceAvailable = await isServiceAvailable(serviceName)
		}
	}

	async function loadTriggers() {
		if (!$workspaceStore || !serviceAvailable) return

		loading = true
		try {
			const triggerList = await NativeTriggerService.listNativeTriggers({
				workspace: $workspaceStore,
				serviceName: serviceName
			})

			triggers = triggerList

			$usedTriggerKinds = removeTriggerKindIfUnused(triggers.length, serviceName, $usedTriggerKinds)
		} catch (err: any) {
			console.error('Failed to load triggers:', err)
			sendUserToast(`Failed to load triggers: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function syncTriggers() {
		if (!serviceConfig?.supportsSync) {
			sendUserToast('Sync is not supported for this service')
			return
		}

		try {
			await NativeTriggerService.syncNativeTriggers({
				workspace: $workspaceStore!,
				serviceName: serviceName
			})

			// Sync completed successfully - reload triggers to show changes
			sendUserToast(`Successfully synced ${serviceConfig.serviceDisplayName} triggers`)
			loadTriggers()
		} catch (err: any) {
			sendUserToast(`Failed to sync triggers: ${err.body || err.message}`, true)
		}
	}

	// Load data when workspace is available
	run(() => {
		if ($workspaceStore && $userStore) {
			checkServiceAvailability().then(() => {
				if (serviceAvailable) {
					loadTriggers()
				}
			})
		}
	})

	onMount(() => {
		checkServiceAvailability()
	})
</script>

<NativeTriggerEditor bind:this={editor} service={serviceName} onUpdate={loadTriggers} />
<ShareModal bind:this={shareModal} on:change={loadTriggers} />

<SearchItems
	{filter}
	items={triggers}
	bind:filteredItems
	f={(trigger) => `${formatTriggerDisplayName(trigger)} ${trigger.path} ${trigger.runnable_path}`}
/>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.triggers}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else if !serviceSupported}
	<CenteredPage>
		<div class="max-w-md mx-auto text-center py-12">
			<AlertTriangle class="mx-auto h-12 w-12 text-yellow-500 mb-4" />
			<h2 class="text-lg font-semibold text-primary mb-2"> Service Not Supported </h2>
			<p class="text-secondary mb-4">
				The service "{serviceName}" is not supported for native triggers.
			</p>
			<p class="text-sm text-tertiary"> Supported services: nextcloud </p>
		</div>
	</CenteredPage>
{:else}
	<CenteredPage>
		<PageHeader
			title="{serviceConfig?.serviceDisplayName || serviceName} Triggers"
			tooltip="Native triggers managed externally by {serviceConfig?.serviceDisplayName ||
				serviceName}. These are more efficient than regular triggers as they're handled directly by the service provider."
		>
			{#if serviceAvailable}
				<div class="flex gap-2">
					{#if serviceConfig?.supportsSync}
						<Button size="md" color="light" startIcon={{ icon: RefreshCw }} on:click={syncTriggers}>
							Sync with {serviceConfig.serviceDisplayName}
						</Button>
					{/if}
					<Button
						size="md"
						color="blue"
						startIcon={{ icon: Plus }}
						on:click={() => editor?.openNew()}
					>
						New {serviceConfig?.serviceDisplayName || serviceName} Trigger
					</Button>
				</div>
			{/if}
		</PageHeader>

		{#if !serviceAvailable}
			<Alert
				title="{serviceConfig?.serviceDisplayName || serviceName} Integration Not Available"
				type="warning"
			>
				<div class="flex items-start gap-3">
					<AlertTriangle size={20} class="text-yellow-600 mt-0.5 shrink-0" />
					<div>
						<p class="mb-2">
							{serviceConfig?.serviceDisplayName || serviceName} triggers are not available because the
							{serviceConfig?.serviceDisplayName || serviceName} OAuth2 integration is not configured
							in the instance settings.
						</p>
						<p class="text-sm text-tertiary">
							Please contact your administrator to configure {serviceConfig?.serviceDisplayName ||
								serviceName} OAuth2 in the instance settings to enable this feature.
						</p>
					</div>
				</div>
			</Alert>
		{:else}
			<div class="w-full h-full flex flex-col">
				<div class="w-full pb-4 pt-6">
					<input
						type="text"
						placeholder="Search {serviceConfig?.serviceDisplayName || serviceName} triggers"
						bind:value={filter}
						class="search-item"
					/>
				</div>

				{#if loading}
					<div class="text-center py-8">
						<div class="text-tertiary"
							>Loading {serviceConfig?.serviceDisplayName || serviceName} triggers...</div
						>
					</div>
				{:else if !triggers?.length}
					<div class="text-center py-8">
						<div class="text-tertiary mb-4"
							>No {serviceConfig?.serviceDisplayName || serviceName} triggers found</div
						>
						<Button
							size="md"
							color="blue"
							startIcon={{ icon: Plus }}
							on:click={() => editor?.openNew()}
						>
							Create your first {serviceConfig?.serviceDisplayName || serviceName} trigger
						</Button>
					</div>
				{:else if filteredItems?.length}
					<NativeTriggerTable
						service={serviceName}
						triggers={filteredItems}
						{loading}
						onEdit={(trigger) => editor?.openEdit(trigger)}
						onSync={syncTriggers}
						{shareModal}
					/>
				{:else}
					<NoItemFound />
				{/if}
			</div>
		{/if}
	</CenteredPage>
{/if}
