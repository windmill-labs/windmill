<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import {
		CheckCircle2,
		AlertCircle,
		RefreshCw,
		CheckSquare2,
		AlertTriangle,
		Save
	} from 'lucide-svelte'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount } from 'svelte'
	import { instanceSettingsSelectedTab } from '$lib/stores'
	import { goto } from '$app/navigation'
	import { superadmin, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Section from '$lib/components/Section.svelte'

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

	let initialMuteSettings = muteSettings

	async function saveMuteSettings() {
		// Workspace
		await SettingService.workspaceMuteCriticalAlertsUi({
			workspace: $workspaceStore!,
			requestBody: {
				mute_critical_alerts: muteSettings.workspace
			}
		})
		sendUserToast(
			`Critical alert UI mute setting for workspace is set to ${muteSettings.workspace}\nreloading page...`
		)

		// Global
		if ($superadmin) {
			await SettingService.setGlobal({
				key: 'critical_alert_mute_ui',
				requestBody: { value: muteSettings.global }
			})
		}

		// reload page after change of setting
		setTimeout(() => {
			window.location.reload()
		}, 3000)
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
</script>

<div>
	<div class="grid grid-cols-3 gap-4 col-start-3">
		<div class="pt-1 col-span-2">
			{#if !hasCriticalAlertChannels && $superadmin}
				<div class="flex flex-row pb-4">
					<AlertTriangle color="orange" class="w-6 h-6 mr-2" />
					<p>
						No critical alert channels are set up. Go to the
						<a href="/#superadmin-settings" on:click|preventDefault={goToCoreTab}
							>Instance Settings</a
						>
						page to configure critical alert channels.
					</p>
				</div>
			{/if}
		</div>

		<div class="flex flex-col justify-between col-start-3">
			<div class="flex flex-row justify-end mt-[-38px] pb-3">
				<Button
					color="green"
					startIcon={{ icon: CheckSquare2 }}
					size="xs"
					disabled={numUnacknowledgedCriticalAlerts === 0}
					on:click={acknowledgeAll}
				>
					Acknowledge All</Button
				>
			</div>

			<Section
				label="Mute Settings"
				collapsable={true}
				tooltip="Show critical alert UI notifications"
				small={true}
			>
				{#if $superadmin}
					<div class="flex flex-row pb-1">
						<Toggle
							bind:checked={muteSettings.global}
							options={{ right: 'Mute critical alerts instance wide' }}
							size="xs"
						/>
					</div>
				{/if}

				<div class="flex flex-row pb-1">
					<Toggle
						bind:checked={muteSettings.workspace}
						options={{ right: 'Mute critical alerts for current workspace' }}
						size="xs"
					/>
				</div>
				<div class="flex flex-row">
					<Button
						disabled={initialMuteSettings.global === muteSettings.global &&
							initialMuteSettings.workspace === muteSettings.workspace}
						size="xs2"
						color="green"
						on:click={saveMuteSettings}
						startIcon={{ icon: Save }}
					>
						Save
					</Button>
				</div>
			</Section>

			<div class="pt-2 flex justify-between items-center">
				<div class="pr-2">
					<Toggle
						bind:checked={hideAcknowledged}
						on:change={refreshAlerts}
						options={{ right: 'Hide Acknowledged' }}
						size="xs"
					/>
				</div>
				<button
					class="mb-1 p-2 rounded-full hover:bg-gray-200"
					on:click={refreshAlerts}
					disabled={loading}
				>
					<RefreshCw class={loading ? 'animate-spin ' : ''} size="20" />
				</button>
			</div>
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
	<div class="flex flex-1 pt-2 gap-x-4 justify-end">
		<Button size="xs2" on:click={goToPreviousPage} disabled={page <= 1}>Previous</Button>
		<span>Page {page}</span>
		<Button size="xs2" on:click={goToNextPage} disabled={!hasMore}>Next</Button>
	</div>

	{#if alerts.length === 0}
		<p class="text-center text-gray-500 mt-4">No critical alerts available.</p>
	{/if}
</div>
