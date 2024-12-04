<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount } from 'svelte'
	import { devopsRole, instanceSettingsSelectedTab, superadmin } from '$lib/stores'
	import { goto } from '$app/navigation'
	import List from '$lib/components/common/layout/List.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import CriticalAlertTable from './CriticalAlertTable.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

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

	onMount(() => {
		refreshAlerts()
	})

	// Pagination
	let page = 1
	let pageSize = 10
	let hasMore = true

	let hideAcknowledged = false
	let workspaceContext = false

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
			filteredAlerts = res.alerts
			updateHasUnacknowledgedCriticalAlerts()
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
		updateHasUnacknowledgedCriticalAlerts()
		await getTotalNumber()
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
		instanceSettingsSelectedTab.set('Core')
	}

	function onFiltersChange() {
		getAlerts(true)
		getTotalNumber()
	}

	// Update filter change handlers
	$: hideAcknowledged, workspaceContext, onFiltersChange()

	let totalNumberOfAlerts = 0
	async function getTotalNumber() {
		loading = true
		const res = await getCriticalAlerts({
			page: 1,
			pageSize: 1000,
			acknowledged: hideAcknowledged ? false : undefined
		})
		totalNumberOfAlerts = res.total_rows
		loading = false
	}
</script>

<List gap="sm">
	{#if !hasCriticalAlertChannels && $superadmin}
		<div class="w-full">
			<Alert title="No critical alert channels are set up" type="warning" size="xs">
				Go to the
				<a href="/#superadmin-settings" on:click|preventDefault={goToCoreTab}>Instance Settings</a>
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
							options={{ left: `Workspace only` }}
							size="xs"
						/>
					{/if}

					<Toggle
						bind:checked={hideAcknowledged}
						options={{ left: 'Hide Acknowledged' }}
						size="xs"
					/>

					<div class="text-xs text-tertiary"
						>{`Results (${totalNumberOfAlerts === 1000 ? '1000+' : totalNumberOfAlerts})`}
					</div>
				</List>
			</div>

			<List horizontal gap="md" justify="end">
				<RefreshButton {loading} on:click={refreshAlerts} />
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
