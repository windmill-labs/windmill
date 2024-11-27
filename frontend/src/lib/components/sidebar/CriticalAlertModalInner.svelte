<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SettingService } from '$lib/gen'
	import { CheckCircle2, AlertCircle, CheckSquare2, AlertTriangle } from 'lucide-svelte'
	import type { CriticalAlert } from '$lib/gen'
	import { onMount } from 'svelte'
	import { devopsRole, workspaceStore, instanceSettingsSelectedTab, superadmin } from '$lib/stores'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import List from '$lib/components/common/layout/List.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'

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
		<div class="overflow-y-auto max-h-1/2">
			<table class="min-w-full w-full">
				<thead class="bg-gray-600 text-white sticky top-0 z-10">
					<tr>
						<th class="w-[60px] px-4 py-2 text-center">Type</th>
						<th class="px-4 py-2 text-center">Message</th>
						<th class="w-[150px] px-4 py-2 text-center">Created At</th>
						{#if $devopsRole}
							<th class="w-[80px] px-4 py-2 text-center">Workspace</th>
						{/if}
						<th class="w-[180px] px-4 py-2 text-center">Acknowledge</th>
					</tr>
				</thead>
				<tbody>
					{#each alerts as { id, alert_type, message, created_at, acknowledged, workspace_id }}
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
								{#if $devopsRole}
									<td class="border px-4 py-2 w-[150px]"
										>{workspace_id ? workspace_id : 'global'}</td
									>
								{/if}
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
	</div>

	<div class="w-full">
		<List horizontal gap="md" justify="end">
			<Button size="xs2" on:click={goToPreviousPage} disabled={page <= 1}>Previous</Button>
			<span>Page {page}</span>
			<Button size="xs2" on:click={goToNextPage} disabled={!hasMore}>Next</Button>
		</List>
	</div>

	{#if alerts.length === 0}
		<p class="text-center text-gray-500 mt-4">No critical alerts available.</p>
	{/if}
</List>
