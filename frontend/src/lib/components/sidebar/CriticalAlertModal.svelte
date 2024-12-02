<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import CriticalAlertModalInner from './CriticalAlertModalInner.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		workspaceStore,
		isCriticalAlertsUIOpen,
		devopsRole,
		userStore,
		superadmin
	} from '$lib/stores'
	import Modal2 from '../common/modal/Modal2.svelte'
	import { Button, Popup } from '$lib/components/common'
	import List from '$lib/components/common/layout/List.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { BellOff, Bell, ExternalLink, Settings } from 'lucide-svelte'
	import { base } from '$lib/base'
	import Notification from '$lib/components/common/alert/Notification.svelte'

	export let open: boolean = false
	export let numUnacknowledgedCriticalAlerts: number = 0
	export let muteSettings
	let workspaceContext = false

	$: {
		setupApiFunctions(workspaceContext)
	}

	function setupApiFunctions(_ctx?) {
		getCriticalAlerts = withSuperadminLogic(
			SettingService.getCriticalAlerts,
			SettingService.workspaceGetCriticalAlerts
		)

		acknowledgeCriticalAlert = withSuperadminLogic(
			SettingService.acknowledgeCriticalAlert,
			SettingService.workspaceAcknowledgeCriticalAlert
		)

		acknowledgeAllCriticalAlerts = withSuperadminLogic(
			SettingService.acknowledgeAllCriticalAlerts,
			SettingService.workspaceAcknowledgeAllCriticalAlerts
		)
	}

	$: isCriticalAlertsUIOpen.set(open)
	$: if ($isCriticalAlertsUIOpen) open = $isCriticalAlertsUIOpen

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false

	const withSuperadminLogic = (superadminFunction, workspaceFunction) => {
		return async (params = {}) => {
			if (!$devopsRole || workspaceContext) {
				return workspaceFunction({
					...params,
					workspace: $workspaceStore
				})
			} else {
				return superadminFunction(params)
			}
		}
	}

	let getCriticalAlerts
	let acknowledgeCriticalAlert
	let acknowledgeAllCriticalAlerts

	setupApiFunctions()

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
				pageSize: 1000,
				acknowledged: false
			})
			if (
				numUnacknowledgedCriticalAlerts === 0 &&
				unacknowledged.length > 0 &&
				sendToast &&
				(($devopsRole && !muteSettings.global) || (!$devopsRole && !muteSettings.workspace))
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

<Modal2 bind:isOpen={open} title="Critical Alerts" target="#content" fixedSize="lg">
	<svelte:fragment slot="header-left">
		<Notification notificationCount={numUnacknowledgedCriticalAlerts} notificationLimit={9999} />
	</svelte:fragment>
	<svelte:fragment slot="header-right">
		<List horizontal>
			{#if $superadmin || $userStore?.is_admin}
				<Popup
					floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
					target="#mute-settings-button"
					preventPopupClosingOnClickInside={true}
				>
					<svelte:fragment slot="button">
						<div id="mute-settings-button">
							<Button variant="border" color="light" nonCaptureEvent>
								{#if muteSettings.global || muteSettings.workspace}
									<BellOff size="16" />
								{:else}
									<Bell size="16" />
								{/if}
							</Button>
						</div>
					</svelte:fragment>
					<List justify="start">
						<div class="w-full">
							{#if $superadmin}
								<Toggle
									bind:checked={muteSettings.global}
									options={{
										right: 'Automatically acknowledge critical alerts instance wide'
									}}
									size="xs"
									stopPropagation={true}
								/>
							{/if}
						</div>

						<div class="w-full">
							<Toggle
								bind:checked={muteSettings.workspace}
								options={{
									right: 'Automatically acknowledge critical alerts for current workspace'
								}}
								size="xs"
								stopPropagation={true}
							/>
						</div>
					</List>
				</Popup>
			{/if}

			{#if $superadmin}
				<Popup
					floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
					target="#settings-button"
				>
					<svelte:fragment slot="button">
						<div id="settings-button">
							<Button variant="border" color="light" nonCaptureEvent>
								<Settings size="16" />
							</Button>
						</div>
					</svelte:fragment>
					<List justify="start" gap="none">
						<div class="w-full">
							<Button
								size="xs"
								color="light"
								href="{base}/?workspace=admins#superadmin-settings"
								target="_blank"
							>
								<div class="w-full">
									<List horizontal justify="between" gap="sm">
										<div>Instance Critical Alert Settings</div>
										<ExternalLink size="16" />
									</List>
								</div>
							</Button>
						</div>
						<div class="w-full">
							<Button
								size="xs"
								color="light"
								href="{base}/workspace_settings?tab=error_handler"
								target="_blank"
							>
								Workspace Critical Alert Settings <ExternalLink size="16" />
							</Button>
						</div>
					</List>
				</Popup>
			{:else}
				<Button
					size="xs"
					color="light"
					variant="border"
					href="{base}/workspace_settings?tab=error_handler"
					target="_blank"
				>
					<List horizontal justify="between" gap="sm">
						<Settings size="16" />
						<ExternalLink size="16" />
					</List>
				</Button>
			{/if}
		</List>
	</svelte:fragment>

	<CriticalAlertModalInner
		{numUnacknowledgedCriticalAlerts}
		{updateHasUnacknowledgedCriticalAlerts}
		{getCriticalAlerts}
		{acknowledgeCriticalAlert}
		{acknowledgeAllCriticalAlerts}
		{muteSettings}
	/>
</Modal2>
