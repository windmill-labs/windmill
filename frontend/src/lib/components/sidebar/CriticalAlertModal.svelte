<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import CriticalAlertModalInner from './CriticalAlertModalInner.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	export let open: boolean = false
	export let numUnacknowledgedCriticalAlerts: number = 0

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false

	onMount(() => {
		updateHasUnacknowledgedCriticalAlerts(true)
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
			const unacknowledged = await SettingService.getCriticalAlerts({
				page: 1,
				pageSize: 10,
				acknowledged: false
			})

			if (numUnacknowledgedCriticalAlerts === 0 && unacknowledged.length > 0 && sendToast) {
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
		await SettingService.acknowledgeCriticalAlert({ id })
		updateHasUnacknowledgedCriticalAlerts()
	}
</script>

<CriticalAlertModalInner bind:open {updateHasUnacknowledgedCriticalAlerts} />
