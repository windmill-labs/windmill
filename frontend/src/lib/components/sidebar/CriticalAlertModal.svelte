<script lang="ts">
	import Modal from '../common/modal/Modal.svelte'
	import Button from '../common/button/Button.svelte'
	import { SettingService } from '$lib/gen'
	import { CheckCircle2, AlertCircle, RefreshCw, EyeOff, Eye, CheckSquare2 } from 'lucide-svelte'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount, onDestroy, beforeUpdate } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	export let open: boolean = false
	export let hasUnaknowledgedCriticalAlerts: boolean = false

	let alerts: CriticalAlert[] = []

	let checkForNewAlertsInterval:  ReturnType<typeof setInterval>;
	let checkingForNewAlerts = false
	let isRefreshing = false
	let previousOpen = open

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

	function toggleHideAcknowledged() {
		hideAcknowledged = !hideAcknowledged
		getAlerts(true)
	}

	async function fetchAlerts(pageNumber: number) {
		console.log('Fetching alerts', pageNumber)
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
		console.log('Getting alerts', reset)
		if (reset) page = 1
		await fetchAlerts(page)
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
				pageSize: 1,
				acknowledged: false
			})

			if (!hasUnaknowledgedCriticalAlerts && unaknowledged.length > 0 && sendToast) {
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
			hasUnaknowledgedCriticalAlerts = unaknowledged.length > 0
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
		console.log('Refreshing alerts')
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
</script>

{#if hasUnaknowledgedCriticalAlerts}
	<button
		on:click={() => (open = true)}
		class="fixed top-4 right-4 rounded-full bg-red-600 p-1 z-50 shadow-lg hover:bg-red-700 active:bg-red-800 focus:outline-none focus:ring-2 focus:ring-red-500"
		aria-label="Toggle Critical Alerts Modal"
	>
		<AlertCircle strokeWidth="3" class="animate-pulse" color="white" size="25" />
	</button>
{/if}

<Modal bind:open title="Critical Alerts" cancelText="Close" style="max-width: 66%;">
	<!-- Row of action buttons above the table -->
	<div class="flex justify-between items-center mb-4">
		<div class="flex space-x-2">
			<Button color="green" startIcon={{ icon: CheckSquare2 }} size="sm" on:click={acknowledgeAll}
				>Acknowledge All</Button
			>
			<Button
				startIcon={hideAcknowledged ? { icon: Eye } : { icon: EyeOff }}
				size="sm"
				on:click={toggleHideAcknowledged}
			>
				{hideAcknowledged ? 'Show Acknowledged' : 'Hide Acknowledged'}
			</Button>
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
	</div>

	<!-- Table of alerts with scrollable body -->
	<div class="overflow-y-auto max-h-1/2">
		<table class="min-w-full bg-white w-full">
			<thead class="bg-gray-600 text-white sticky top-0 z-10">
				<tr>
					<th class="w-[60px] px-4 py-2 text-center">Type</th>
					<!-- Fixed width -->
					<th class="px-4 py-2 text-center">Message</th>
					<!-- Flexible width -->
					<th class="w-[150px] px-4 py-2 text-center">Created At</th>
					<!-- Fixed width -->
					<th class="w-[180px] px-4 py-2 text-center">Acknowledge</th>
					<!-- Fixed width -->
				</tr>
			</thead>
			<tbody>
				{#each alerts as { id, alert_type, message, created_at, acknowledged }}
					{#if !hideAcknowledged || !acknowledged}
						<tr class="bg-gray-100 text-center">
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
