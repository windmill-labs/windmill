<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import CriticalAlertModalInner from './CriticalAlertModalInner.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	export let open: boolean = false
	export let numUnaknowledgedCriticalAlerts: number = 0

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false

	onMount(() => {
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
			}
			numUnaknowledgedCriticalAlerts = unaknowledged.length
		} finally {
			checkingForNewAlerts = false
		}
	}

	async function acknowledgeAlert(id: number) {
		await SettingService.acknowledgeCriticalAlert({ id })
	}
</script>

<CriticalAlertModalInner bind:open {updateHasUnaknowledgedCriticalAlerts} />
