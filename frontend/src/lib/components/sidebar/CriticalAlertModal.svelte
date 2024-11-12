<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import CriticalAlertModalInner from './CriticalAlertModalInner.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { superadmin, workspaceStore, isCriticalAlertsUIOpen } from '$lib/stores'
	import Modal from '../common/modal/Modal.svelte'

	export let open: boolean = false
	export let numUnacknowledgedCriticalAlerts: number = 0
	export let muteSettings

	$: isCriticalAlertsUIOpen.set(open)
	$: if ($isCriticalAlertsUIOpen) open = $isCriticalAlertsUIOpen

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false

	const withSuperadminLogic = (superadminFunction, workspaceFunction) => {
		return async (params = {}) => {
			if (!$superadmin) {
				return workspaceFunction({
					...params,
					workspace: $workspaceStore
				})
			} else {
				return superadminFunction(params)
			}
		}
	}

	const getCriticalAlerts = withSuperadminLogic(
		SettingService.getCriticalAlerts,
		SettingService.workspaceGetCriticalAlerts
	)

	const acknowledgeCriticalAlert = withSuperadminLogic(
		SettingService.acknowledgeCriticalAlert,
		SettingService.workspaceAcknowledgeCriticalAlert
	)

	const acknowledgeAllCriticalAlerts = withSuperadminLogic(
		SettingService.acknowledgeAllCriticalAlerts,
		SettingService.workspaceAcknowledgeAllCriticalAlerts
	)

	onMount(async () => {
		await updateHasUnacknowledgedCriticalAlerts(false)
		checkForNewAlertsInterval = setInterval(() => {
			updateHasUnacknowledgedCriticalAlerts(true)
		}, 15000)
	})

	onDestroy(() => {
		clearInterval(checkForNewAlertsInterval)
	})

	async function updateHasUnacknowledgedCriticalAlerts(sendToast: boolean = false) {
		if (checkingForNewAlerts) return
		checkingForNewAlerts = true
		try {
			const unacknowledged = await getCriticalAlerts({
				page: 1,
				pageSize: 10,
				acknowledged: false
			})
			if (
				numUnacknowledgedCriticalAlerts === 0 &&
				unacknowledged.length > 0 &&
				sendToast &&
				(($superadmin && !muteSettings.global) || (!$superadmin && !muteSettings.workspace))
			) {
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
								if (unacknowledged[0].id) acknowledgeAlert(unacknowledged[0].id)
							}
						}
					],
					unacknowledged[0].message,
					10000
				)
			}
			numUnacknowledgedCriticalAlerts = unacknowledged.length
		} finally {
			checkingForNewAlerts = false
		}
	}

	async function acknowledgeAlert(id: number) {
		await acknowledgeCriticalAlert({ id })
		updateHasUnacknowledgedCriticalAlerts()
	}
</script>

<Modal bind:open title="Critical Alerts" cancelText="Close" style="max-width: 66%;">
	<CriticalAlertModalInner
		{numUnacknowledgedCriticalAlerts}
		{updateHasUnacknowledgedCriticalAlerts}
		{getCriticalAlerts}
		{acknowledgeCriticalAlert}
		{acknowledgeAllCriticalAlerts}
		{muteSettings}
	/>
</Modal>
