<script lang="ts">
	import { run } from 'svelte/legacy'
	import { NativeTriggerService } from '$lib/gen'
	import type { NativeTrigger } from '$lib/gen'
	import type { NativeTriggerWithWrite } from '$lib/components/triggers/native/utils'
	import {
		isServiceAvailable,
		formatTriggerDisplayName
	} from '$lib/components/triggers/native/utils'
	import NativeTriggerTable from '$lib/components/triggers/native/NativeTriggerTable.svelte'
	import NativeTriggerEditor from '$lib/components/triggers/native/NativeTriggerEditor.svelte'
	import { sendUserToast, removeTriggerKindIfUnused } from '$lib/utils'
	import {
		userStore,
		workspaceStore,
		userWorkspaces,
		usedTriggerKinds
	} from '$lib/stores'
	import { onMount } from 'svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Button, Alert } from '$lib/components/common'
	import { Plus, AlertTriangle } from 'lucide-svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'

	let triggers: NativeTrigger[] = $state([])
	let loading = $state(true)
	let serviceAvailable = $state(false)
	let editor: NativeTriggerEditor
	let shareModal: ShareModal | undefined = $state()

	let filteredItems: NativeTriggerWithWrite[] = $state([])
	let filter = $state('')

	async function checkServiceAvailability() {
		if ($workspaceStore) {
			serviceAvailable = await isServiceAvailable('nextcloud')
		}
	}


	async function loadTriggers() {
		if (!$workspaceStore || !serviceAvailable) return

		loading = true
		try {
			const triggerList = await NativeTriggerService.listNativeTriggers({
				workspace: $workspaceStore,
				serviceName: 'nextcloud'
			})

			triggers = triggerList

			$usedTriggerKinds = removeTriggerKindIfUnused(triggers.length, 'nextcloud', $usedTriggerKinds)
		} catch (err: any) {
			console.error('Failed to load triggers:', err)
			sendUserToast(`Failed to load triggers: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function syncTriggers() {
		// Mock sync implementation - replace with actual sync call when available
		sendUserToast('Sync functionality will be implemented once backend sync endpoint is ready')
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

<NativeTriggerEditor bind:this={editor} service="nextcloud" onUpdate={loadTriggers} />
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
{:else}
	<CenteredPage>
		<PageHeader
			title="Nextcloud Triggers"
			tooltip="Native triggers managed externally by Nextcloud. These are more efficient than regular triggers as they're handled directly by the service provider."
		>
			{#if serviceAvailable}
				<Button
					size="md"
					color="blue"
					startIcon={{ icon: Plus }}
					on:click={() => editor?.openNew()}
				>
					New Nextcloud Trigger
				</Button>
			{/if}
		</PageHeader>

		{#if !serviceAvailable}
			<Alert title="Nextcloud Integration Not Available" type="warning">
				<div class="flex items-start gap-3">
					<AlertTriangle size={20} class="text-yellow-600 mt-0.5 shrink-0" />
					<div>
						<p class="mb-2">
							Nextcloud triggers are not available because the Nextcloud OAuth2 integration is not
							configured in the instance settings.
						</p>
						<p class="text-sm text-tertiary">
							Please contact your administrator to configure Nextcloud OAuth2 in the instance
							settings to enable this feature.
						</p>
					</div>
				</div>
			</Alert>
		{:else}
			<div class="w-full h-full flex flex-col">
				<div class="w-full pb-4 pt-6">
					<input
						type="text"
						placeholder="Search Nextcloud triggers"
						bind:value={filter}
						class="search-item"
					/>
				</div>

				{#if loading}
					<div class="text-center py-8">
						<div class="text-tertiary">Loading Nextcloud triggers...</div>
					</div>
				{:else if !triggers?.length}
					<div class="text-center py-8">
						<div class="text-tertiary mb-4">No Nextcloud triggers found</div>
						<Button
							size="md"
							color="blue"
							startIcon={{ icon: Plus }}
							on:click={() => editor?.openNew()}
						>
							Create your first Nextcloud trigger
						</Button>
					</div>
				{:else if filteredItems?.length}
					<NativeTriggerTable
						service="nextcloud"
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
