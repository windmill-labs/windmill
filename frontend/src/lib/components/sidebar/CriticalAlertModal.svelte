<script lang="ts">
	import { run } from 'svelte/legacy'

	import { onMount, onDestroy } from 'svelte'
	import CriticalAlertModalInner from './CriticalAlertModalInner.svelte'
	import { SettingService, type CriticalAlert } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		workspaceStore,
		isCriticalAlertsUIOpen,
		devopsRole,
		userStore,
		superadmin
	} from '$lib/stores'
	import Modal2 from '../common/modal/Modal2.svelte'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import List from '$lib/components/common/layout/List.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { BellOff, Bell, ExternalLink, Settings } from 'lucide-svelte'
	import { base } from '$lib/base'
	import Notification from '$lib/components/common/alert/Notification.svelte'

	interface Props {
		open?: boolean
		numUnacknowledgedCriticalAlerts?: number
		muteSettings: any
	}

	let {
		open = $bindable(false),
		numUnacknowledgedCriticalAlerts = $bindable(0),
		muteSettings = $bindable()
	}: Props = $props()
	let workspaceContext = $state(false)
	let childRef: any = $state() // TODO type

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

	let getCriticalAlerts = $state()
	let acknowledgeCriticalAlert: any = $state() // TODO type
	let acknowledgeAllCriticalAlerts = $state()

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

	async function saveWorkSpaceMuteSetting() {
		await SettingService.workspaceMuteCriticalAlertsUi({
			workspace: $workspaceStore!,
			requestBody: {
				mute_critical_alerts: muteSettings.workspace
			}
		})
		sendUserToast(
			`Critical alert UI mute settings changed.\nPlease reload page for UI changes to take effect.`
		)
		childRef.refreshAlerts()
	}
	async function saveGlobalMuteSetting() {
		await SettingService.setGlobal({
			key: 'critical_alert_mute_ui',
			requestBody: { value: muteSettings.global }
		})
		sendUserToast(
			`Critical alert UI mute settings changed.\nPlease reload page for UI changes to take effect.`
		)
		childRef.refreshAlerts()
	}

	async function updateHasUnacknowledgedCriticalAlerts(sendToast: boolean = false) {
		if (typeof document !== 'undefined' && document.visibilityState !== 'visible') {
			return
		}
		if (checkingForNewAlerts) return
		checkingForNewAlerts = true
		try {
			const params = {
				page: 1,
				pageSize: 1000,
				acknowledged: false
			}
			let unacknowledged: CriticalAlert[] = []
			if (!$devopsRole && $workspaceStore) {
				const res = await SettingService.workspaceGetCriticalAlerts({
					...params,
					workspace: $workspaceStore
				})
				unacknowledged = res.alerts ?? []
			} else {
				const res = await SettingService.getCriticalAlerts(params)
				unacknowledged = res.alerts ?? []
			}

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
		} catch (e) {
			console.error('Error fetching critical alerts', e)
			sendUserToast('Error fetching critical alerts', true)
		} finally {
			checkingForNewAlerts = false
		}
	}

	async function acknowledgeAlert(id: number) {
		await acknowledgeCriticalAlert({ id })
		updateHasUnacknowledgedCriticalAlerts()
	}
	run(() => {
		setupApiFunctions(workspaceContext)
	})
	run(() => {
		if ($isCriticalAlertsUIOpen) open = $isCriticalAlertsUIOpen
	})
	run(() => {
		isCriticalAlertsUIOpen.set(open)
	})
</script>

<Modal2 bind:isOpen={open} title="Critical Alerts" target="#content" fixedSize="lg">
	<!-- @migration-task: migrate this slot by hand, `header-left` is an invalid identifier -->
	<svelte:fragment slot="header-left">
		<Notification notificationCount={numUnacknowledgedCriticalAlerts} notificationLimit={9999} />
	</svelte:fragment>
	<!-- @migration-task: migrate this slot by hand, `header-right` is an invalid identifier -->
	<svelte:fragment slot="header-right">
		<List horizontal>
			{#if $superadmin || $userStore?.is_admin}
				<Popover
					floatingConfig={{ strategy: 'fixed', placement: 'bottom-end' }}
					portal="#mute-settings-button"
					contentClasses="p-4"
				>
					{#snippet trigger()}
						<div id="mute-settings-button">
							<Button variant="default" nonCaptureEvent>
								{#if muteSettings.global || muteSettings.workspace}
									<BellOff size="16" />
								{:else}
									<Bell size="16" />
								{/if}
							</Button>
						</div>
					{/snippet}
					{#snippet content()}
						<List justify="start">
							<div class="w-full">
								{#if $superadmin}
									<Toggle
										on:change={saveGlobalMuteSetting}
										bind:checked={muteSettings.global}
										options={{
											right: 'Automatically acknowledge critical alerts instance wide'
										}}
										size="xs"
									/>
								{/if}
							</div>

							<div class="w-full">
								<Toggle
									on:change={saveWorkSpaceMuteSetting}
									bind:checked={muteSettings.workspace}
									options={{
										right: 'Automatically acknowledge critical alerts for current workspace'
									}}
									size="xs"
								/>
							</div>
						</List>
					{/snippet}
				</Popover>
			{/if}

			{#if $superadmin}
				<Popover
					floatingConfig={{ strategy: 'fixed', placement: 'bottom-end' }}
					portal="#settings-button"
					contentClasses="p-4"
				>
					{#snippet trigger()}
						<div id="settings-button">
							<Button variant="default" nonCaptureEvent>
								<Settings size="16" />
							</Button>
						</div>
					{/snippet}
					{#snippet content()}
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
					{/snippet}
				</Popover>
			{:else}
				<Button
					size="xs"
					variant="default"
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
		bind:workspaceContext
		{numUnacknowledgedCriticalAlerts}
		{updateHasUnacknowledgedCriticalAlerts}
		{getCriticalAlerts}
		{acknowledgeCriticalAlert}
		{acknowledgeAllCriticalAlerts}
		bind:this={childRef}
	/>
</Modal2>
