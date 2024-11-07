<script lang="ts">
	import Modal from '../common/modal/Modal.svelte'
	import Button from '../common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { CheckCircle2, AlertCircle, RefreshCw, CheckSquare2, AlertTriangle } from 'lucide-svelte'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount, onDestroy, beforeUpdate } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { instanceSettingsSelectedTab } from '$lib/stores'
	import { goto } from '$app/navigation'

	export let open: boolean = false
	export let numUnaknowledgedCriticalAlerts: number = 0

	let alerts: CriticalAlert[] = []

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false
	let isRefreshing = false
	let previousOpen = open
	let hasCriticalAlertChannels = false

	$: loading = isRefreshing

	beforeUpdate(() => {
		if (open && !previousOpen) {
			refreshAlerts()
		}
		previousOpen = open
	})

	// Pagination
	let page = 1
	let pageSize = 10
	let hasMore = true

	let hideAcknowledged = false

	async function acknowledgeAll() {
		await SettingService.acknowledgeAllCriticalAlerts()
		getAlerts(false)
	}

	async function fetchAlerts(pageNumber: number) {
		isRefreshing = true
		try {
			const newAlerts = await SettingService.getCriticalAlerts({
				page: pageNumber,
				pageSize: pageSize,
				acknowledged: hideAcknowledged ? false : undefined
			})

			alerts = newAlerts
			hasMore = newAlerts.length === pageSize
			page = pageNumber
			updateHasUnaknowledgedCriticalAlerts()
		} finally {
			setTimeout(() => {
				isRefreshing = false
			}, 500)
		}
	}

	async function getAlerts(reset?: boolean) {
		if (reset) page = 1
		await fetchAlerts(page)
	}

	async function checkCriticalAlertChannels() {
		const channels = await SettingService.getGlobal({key: "critical_error_channels"}) as any[]
		hasCriticalAlertChannels = channels && channels.length > 0
	}

	onMount(() => {
		updateHasUnaknowledgedCriticalAlerts()
		checkForNewAlertsInterval = setInterval(() => {
			updateHasUnaknowledgedCriticalAlerts(true)
		}, 15000)
	})

	onDestroy(() => {
		clearInterval(checkForNewAlertsInterval)
	})

	async function updateHasUnaknowledgedCriticalAlerts(sendToast: boolean = false) {
		if (checkingForNewAlerts) return
		checkingForNewAlerts = true

		try {
			// check if there is at least one unaknowledged alert
			const unaknowledged = await SettingService.getCriticalAlerts({
				page: 1,
				pageSize: 10,
				acknowledged: false
			})

			if (numUnaknowledgedCriticalAlerts === 0 && unaknowledged.length > 0 && sendToast) {
				sendUserToast(
					'Critical Alert:',
					true,
					[
						{
							label: 'View',
							callback: () => {
								open = true
							}
						},
						{
							label: 'Acknowledge',
							callback: () => {
								if (unaknowledged[0].id) acknowledgeAlert(unaknowledged[0].id)
							}
						}
					],
					unaknowledged[0].message,
					10000
				)
				if (open && checkingForNewAlerts) getAlerts(false)
			}
			numUnaknowledgedCriticalAlerts = unaknowledged.length
		} finally {
			checkingForNewAlerts = false
		}
	}

	async function acknowledgeAlert(id: number) {
		await SettingService.acknowledgeCriticalAlert({ id })
		getAlerts(false)
	}

	function formatDate(dateString: string | undefined): string {
		if (!dateString) return ''
		const date = new Date(dateString)
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		}).format(date)
	}

	async function refreshAlerts() {
		checkCriticalAlertChannels()
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
</script>

<Modal bind:open title="Critical Alerts" cancelText="Close" style="max-width: 66%;">
	{#if !hasCriticalAlertChannels}
		<div class="flex flex-row pb-4">
			<AlertTriangle color="orange" class="w-6 h-6 mr-2" />
			<p>
				No critical alert channels are set up. Go to the
				<a href="/#superadmin-settings" on:click|preventDefault={goToCoreTab}>Instance Settings</a>
				page to configure critical alert channels.
			</p>
		</div>
	{/if}
	<!-- Row of action buttons above the table -->
	<div class="flex justify-between items-center mb-4">
		<div class="flex space-x-2">
			<Button color="green" startIcon={{ icon: CheckSquare2 }} size="sm" on:click={acknowledgeAll}
				>Acknowledge All</Button
			>
		</div>

		<button class="p-2 rounded-full hover:bg-gray-200" on:click={refreshAlerts} disabled={loading}>
			<RefreshCw class={loading ? 'animate-spin ' : ''} size="20" />
		</button>
	</div>

	<!-- Pagination controls above the table -->
	<div class="flex justify-between items-center mb-2">
		<div class="flex items-center space-x-4">
			<Button size="xs2" on:click={goToPreviousPage} disabled={page <= 1}>Previous</Button>
			<span>Page {page}</span>
			<Button size="xs2" on:click={goToNextPage} disabled={!hasMore}>Next</Button>
		</div>
		<div class="pr-3">
			<Toggle
				bind:checked={hideAcknowledged}
				on:change={refreshAlerts}
				options={{ right: 'Hide Acknowledged' }}
				size="xs"
			/>
		</div>
	</div>

	<!-- Table of alerts with scrollable body -->
	<div class="overflow-y-auto max-h-1/2">
		<table class="min-w-full w-full">
			<thead class="bg-gray-600 text-white sticky top-0 z-10">
				<tr>
					<th class="w-[60px] px-4 py-2 text-center">Type</th>
					<th class="px-4 py-2 text-center">Message</th>
					<th class="w-[150px] px-4 py-2 text-center">Created At</th>
					<th class="w-[180px] px-4 py-2 text-center">Acknowledge</th>
				</tr>
			</thead>
			<tbody>
				{#each alerts as { id, alert_type, message, created_at, acknowledged }}
					{#if !hideAcknowledged || !acknowledged}
						<tr class="bg-gray-100 dark:bg-gray-700 dark:text-white text-center">
							<td class="border px-4 py-2 w-[100px]">
								{#if alert_type === 'recovered_critical_error'}
									<span title="Recovered Critical Alert">
										<CheckCircle2 size="20" color="green" />
									</span>
								{:else}
									<span title="Critical Alert">
										<AlertCircle size="20" color="red" />
									</span>
								{/if}
							</td>
							<td class="border px-4 py-2">{message}</td>
							<!-- Flexible width -->
							<td class="border px-4 py-2 w-[150px]">{formatDate(created_at)}</td>
							<td class="border px-4 py-2 w-[180px]">
								<div class="flex justify-center items-center">
									{#if !acknowledged}
										<Button
											color="green"
											startIcon={{ icon: CheckSquare2 }}
											size="xs2"
											on:click={() => {
												if (id) acknowledgeAlert(id)
											}}>Acknowledge</Button
										>
									{:else}
										<CheckCircle2 size="20" color="green" />
									{/if}
								</div>
							</td>
						</tr>
					{/if}
				{/each}
			</tbody>
		</table>
	</div>

	{#if alerts.length === 0}
		<p class="text-center text-gray-500 mt-4">No critical alerts available.</p>
	{/if}
</Modal>
