<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { CheckSquare2, AlertTriangle } from 'lucide-svelte'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount } from 'svelte'
	import { devopsRole, workspaceStore, instanceSettingsSelectedTab, superadmin } from '$lib/stores'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import List from '$lib/components/common/layout/List.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import CriticalAlertTable from './CriticalAlertTable.svelte'

	export let updateHasUnacknowledgedCriticalAlerts
	export let getCriticalAlerts
	export let acknowledgeCriticalAlert
	export let acknowledgeAllCriticalAlerts
	export let numUnacknowledgedCriticalAlerts

	let alerts: CriticalAlert[] = []

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

	let hideAcknowledged = false

	async function acknowledgeAll() {
		await acknowledgeAllCriticalAlerts()
		getAlerts(false)
	}

	async function fetchAlerts(pageNumber: number) {
		isRefreshing = true
		try {
			const newAlerts = await getCriticalAlerts({
				page: pageNumber,
				pageSize: pageSize,
				acknowledged: hideAcknowledged ? false : undefined
			})

			alerts = newAlerts
			hasMore = newAlerts.length === pageSize
			page = pageNumber
			updateHasUnacknowledgedCriticalAlerts()
		} finally {
			setTimeout(() => {
				isRefreshing = false
			}, 500)
		}
	}

	async function getAlerts(reset?: boolean) {
		if (reset) page = 1
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
			fetchAlerts(page - 1)
		}
	}

	function goToNextPage() {
		if (hasMore) {
			fetchAlerts(page + 1)
		}
	}

	function goToCoreTab() {
		goto('/#superadmin-settings')
		instanceSettingsSelectedTab.set('Core')
	}

	export let workspaceContext = false

	$: {
		workspaceContextChanged(workspaceContext)
	}

	async function workspaceContextChanged(_ctx) {
		await getAlerts(true)
	}

	let popupTarget: HTMLElement
	$: console.log('dbg', popupTarget)
</script>

<List gap="sm">
	<div class="w-full">
		<List horizontal justify="between">
			<div>
				<List horizontal justify="start" gap="md">
					{#if $devopsRole}
						<Toggle
							bind:checked={workspaceContext}
							options={{ left: `Workspace: '${$workspaceStore}' only` }}
							size="xs"
						/>
					{/if}
					{#if !hasCriticalAlertChannels && $superadmin}
						<AlertTriangle color="orange" class="w-6 h-6 mr-2" />
						<p>
							No critical alert channels are set up. Go to the
							<a href="/#superadmin-settings" on:click|preventDefault={goToCoreTab}
								>Instance Settings</a
							>
							page to configure critical alert channels.
						</p>
					{/if}

					<Toggle
						bind:checked={hideAcknowledged}
						on:change={refreshAlerts}
						options={{ left: 'Hide Acknowledged' }}
						size="xs"
					/>
				</List>
			</div>

			<List horizontal gap="md" justify="end">
				<Button
					color="green"
					startIcon={{ icon: CheckSquare2 }}
					size="xs"
					disabled={numUnacknowledgedCriticalAlerts === 0}
					on:click={acknowledgeAll}
				>
					Acknowledge All</Button
				>

				<RefreshButton {loading} on:click={refreshAlerts} />
			</List>
		</List>
	</div>

	<div class="w-full">
		<CriticalAlertTable
			{alerts}
			{acknowledgeAlert}
			{hideAcknowledged}
			{goToNextPage}
			{goToPreviousPage}
			bind:page
			{hasMore}
		/>
	</div>

	{#if alerts.length === 0}
		<p class="text-center text-gray-500 mt-4">No critical alerts available.</p>
	{/if}
</List>
