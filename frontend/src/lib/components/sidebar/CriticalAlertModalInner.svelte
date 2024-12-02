<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount } from 'svelte'
	import { devopsRole, workspaceStore, instanceSettingsSelectedTab, superadmin } from '$lib/stores'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
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

	export let muteSettings = {
		workspace: true,
		global: true
	}

	$: muteSettings
	$: {
		if (
			initialMuteSettings.workspace !== muteSettings.workspace ||
			initialMuteSettings.global !== muteSettings.global
		) {
			saveMuteSettings()
		}
	}

	$: numUnacknowledgedCriticalAlerts >= 0 && getAlerts(true)

	let initialMuteSettings = muteSettings

	async function saveMuteSettings() {
		if (initialMuteSettings.workspace !== muteSettings.workspace) {
			// Workspace
			await SettingService.workspaceMuteCriticalAlertsUi({
				workspace: $workspaceStore!,
				requestBody: {
					mute_critical_alerts: muteSettings.workspace
				}
			})
		}
		if ($superadmin && initialMuteSettings.global !== muteSettings.global) {
			// Global
			await SettingService.setGlobal({
				key: 'critical_alert_mute_ui',
				requestBody: { value: muteSettings.global }
			})
		}
		sendUserToast(
			`Critical alert UI mute settings changed.\nPlease reload page for UI changes to take effect.`
		)
		getAlerts(true)
		initialMuteSettings = { ...muteSettings }
	}

	$: loading = isRefreshing

	onMount(() => {
		refreshAlerts()
		initialMuteSettings = { ...muteSettings }
	})

	// Pagination
	let page = 1
	let pageSize = 10
	let hasMore = true
	let isLoading = false
	let indexMapping: number[] = [0]

	let hideAcknowledged = false
	let workspaceContext = false

	async function acknowledgeAll() {
		await acknowledgeAllCriticalAlerts()
		getAlerts(false)
	}

	async function fetchAlerts(pageNumber: number) {
		isRefreshing = true
		try {
			filteredAlerts = await fetchAndFilterAlerts(pageNumber)
			updateHasUnacknowledgedCriticalAlerts()
		} finally {
			setTimeout(() => {
				isRefreshing = false
			}, 500)
		}
	}

	async function fetchAndFilterAlerts(pageNumber: number) {
		isLoading = true
		try {
			if (indexMapping.length < pageNumber) {
				indexMapping.push((pageNumber - 1) * pageSize)
			}
			let startIndex = indexMapping[pageNumber - 1]
			let filteredResults: CriticalAlert[] = []

			while (filteredResults.length < pageSize) {
				const newAlerts = await getCriticalAlerts({
					page: Math.floor(startIndex / pageSize) + 1,
					pageSize: pageSize
				})

				if (newAlerts.length === 0) {
					hasMore = false
					break
				}

				const filtered = filterAlerts(newAlerts)
				filteredResults.push(...filtered)

				if (filteredResults.length >= pageSize) {
					const lastAlertIndex =
						startIndex +
						newAlerts.findIndex((alert) => alert.id === filteredResults[pageSize - 1]?.id)
					indexMapping[pageNumber] = lastAlertIndex + 1
				}

				startIndex += pageSize

				if (newAlerts.length < pageSize) {
					hasMore = false
					break
				}
			}

			hasMore = filteredResults.length >= pageSize

			return filteredResults.slice(0, pageSize)
		} finally {
			isLoading = false
		}
	}

	async function getAlerts(reset?: boolean) {
		if (reset) {
			page = 1
			indexMapping = [] // Reset the index mapping when filters change
		}
		updateHasUnacknowledgedCriticalAlerts()
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

	async function refreshAlerts() {
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
		indexMapping = []
		getAlerts(true)
		getTotalNumber()
	}

	// Update filter change handlers
	$: hideAcknowledged, workspaceContext, onFiltersChange()

	function filterAlerts(alerts: CriticalAlert[]) {
		let filteredAlerts = [...alerts]
		if (hideAcknowledged) {
			filteredAlerts = filteredAlerts.filter((alert) => alert.acknowledged === false)
		}
		if (workspaceContext) {
			filteredAlerts = filteredAlerts.filter((alert) => alert.workspace_id !== null)
		}
		return filteredAlerts
	}

	let totalNumberOfAlerts = 0
	async function getTotalNumber() {
		loading = true
		let alerts = await getCriticalAlerts({ page: 1, pageSize: 1000 })
		if (hideAcknowledged) {
			alerts = alerts.filter((alert) => alert.acknowledged === false)
		}
		if (workspaceContext) {
			alerts = alerts.filter((alert) => alert.workspace_id !== null)
		}
		totalNumberOfAlerts = alerts.length
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
							on:change={refreshAlerts}
							options={{ left: `Workspace only` }}
							size="xs"
						/>
					{/if}

					<Toggle
						bind:checked={hideAcknowledged}
						on:change={refreshAlerts}
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
