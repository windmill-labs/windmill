<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import type { CriticalAlert } from '$lib/gen'
	import { devopsRole, instanceSettingsSelectedTab, superadmin } from '$lib/stores'
	import { goto } from '$app/navigation'
	import List from '$lib/components/common/layout/List.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import CriticalAlertTable from './CriticalAlertTable.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'

	export let updateHasUnacknowledgedCriticalAlerts
	export let getCriticalAlerts
	export let acknowledgeCriticalAlert
	export let acknowledgeAllCriticalAlerts
	export let numUnacknowledgedCriticalAlerts

	let filteredAlerts: CriticalAlert[] = []

	let isRefreshing = false
	let hasCriticalAlertChannels = true

	$: loading = isRefreshing

	$: if (numUnacknowledgedCriticalAlerts) {
		refreshAlerts()
	}

	// Pagination
	let page = 1
	let pageSize = 10
	let hasMore = true

	let hideAcknowledged = false
	export let workspaceContext = false

	async function acknowledgeAll() {
		await acknowledgeAllCriticalAlerts()
		getAlerts(false)
	}

	async function fetchAlerts(pageNumber: number) {
		isRefreshing = true
		try {
			const res = await getCriticalAlerts({
				page: pageNumber,
				pageSize: pageSize,
				acknowledged: hideAcknowledged ? false : undefined
			})

			hasMore = pageNumber < res.total_pages
			totalNumberOfAlerts = res.total_rows
			filteredAlerts = res.alerts
			updateHasUnacknowledgedCriticalAlerts()
		} catch (e) {
			console.error('Error fetching critical alerts', e)
			sendUserToast('Error fetching critical alerts', true)
		} finally {
			setTimeout(() => {
				isRefreshing = false
			}, 500)
		}
	}

	async function getAlerts(reset?: boolean) {
		if (reset) {
			page = 1
		}
		await fetchAlerts(page)
	}

	async function checkCriticalAlertChannels() {
		const channels = (await SettingService.getGlobal({ key: 'critical_error_channels' })) as any[]
		hasCriticalAlertChannels = channels && channels.length > 0
	}

	async function acknowledgeAlert(id: number) {
		await acknowledgeCriticalAlert({ id })
		getAlerts(false)
	}

	export async function refreshAlerts() {
		if ($superadmin) checkCriticalAlertChannels()
		await getAlerts(true)
	}

	function goToPreviousPage() {
		if (page > 1) {
			page -= 1
			fetchAlerts(page)
		}
	}

	function goToNextPage() {
		if (hasMore) {
			page += 1
			fetchAlerts(page)
		}
	}

	function goToCoreTab() {
		goto('/#superadmin-settings')
		instanceSettingsSelectedTab.set('general')
	}

	function onFiltersChange() {
		getAlerts(true)
	}

	// Update filter change handlers
	$: (hideAcknowledged, workspaceContext, onFiltersChange())

	let totalNumberOfAlerts = 0
</script>

<List gap="sm">
	{#if !hasCriticalAlertChannels && $superadmin}
		<div class="w-full">
			<Alert title="No critical alert channels are set up" type="warning" size="xs">
				Go to the
				<a href="/#superadmin-settings" on:click|preventDefault={goToCoreTab}>Instance settings</a>
				page to configure critical alert channels.
			</Alert>
		</div>
	{/if}

	<div class="w-full">
		<List horizontal justify="between">
			<div class="w-full">
				<List horizontal justify="start" gap="md">
					{#if $devopsRole}
						<Toggle
							bind:checked={workspaceContext}
							options={{ right: `Workspace only` }}
							size="xs"
						/>
					{/if}

					<Toggle bind:checked={hideAcknowledged} options={{ right: 'Non-Acked only' }} size="xs" />
				</List>
			</div>

			<List wFull={false} horizontal gap="md" justify="end">
				<div class="text-xs text-primary whitespace-nowrap"
					>{`${totalNumberOfAlerts === 1000 ? '1000+' : (totalNumberOfAlerts ?? '?')} items`}
				</div>
				<RefreshButton {loading} onClick={refreshAlerts} />
			</List>
		</List>
	</div>

	<CriticalAlertTable
		alerts={filteredAlerts}
		{acknowledgeAlert}
		{hideAcknowledged}
		{goToNextPage}
		{goToPreviousPage}
		bind:page
		{hasMore}
		{acknowledgeAll}
		{numUnacknowledgedCriticalAlerts}
		{pageSize}
	/>
</List>
