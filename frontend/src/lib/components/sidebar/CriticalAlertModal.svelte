<script lang="ts">
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
	let childRef: CriticalAlertModalInner | undefined = $state()

	let checkForNewAlertsInterval: ReturnType<typeof setInterval>
	let checkingForNewAlerts = false

	// The returned closure reads workspaceContext / $workspaceStore / $devopsRole at
	// call time, so the wrappers are stable and never need recreating.
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

	let getCriticalAlerts = $derived(
		withSuperadminLogic(SettingService.getCriticalAlerts, SettingService.workspaceGetCriticalAlerts)
	)
	let acknowledgeCriticalAlert = $derived(
		withSuperadminLogic(
			SettingService.acknowledgeCriticalAlert,
			SettingService.workspaceAcknowledgeCriticalAlert
		)
	)
	let acknowledgeAllCriticalAlerts = $derived(
		withSuperadminLogic(
			SettingService.acknowledgeAllCriticalAlerts,
			SettingService.workspaceAcknowledgeAllCriticalAlerts
		)
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
		childRef?.refreshAlerts()
	}
	async function saveGlobalMuteSetting() {
		await SettingService.setGlobal({
			key: 'critical_alert_mute_ui',
			requestBody: { value: muteSettings.global }
		})
		sendUserToast(
			`Critical alert UI mute settings changed.\nPlease reload page for UI changes to take effect.`
		)
		childRef?.refreshAlerts()
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
		await acknowledgeCriticalAlert?.({ id })
		updateHasUnacknowledgedCriticalAlerts()
	}
	$effect(() => {
		if ($isCriticalAlertsUIOpen) open = $isCriticalAlertsUIOpen
	})
	$effect(() => {
		isCriticalAlertsUIOpen.set(open)
	})
</script>

<Modal2
	bind:isOpen={open}
	title="Critical Alerts"
	target="#content"
	fixedHeight="lg"
	fixedWidth="lg"
>
	{#snippet headerLeft()}
		<Notification notificationCount={numUnacknowledgedCriticalAlerts} notificationLimit={9999} />
	{/snippet}
	{#snippet headerRight()}
		<List horizontal>
			{#if $superadmin || $userStore?.is_admin}
				<!-- Portal to `body` (not the trigger) so toggle clicks don't bubble to the melt
				     trigger and toggle the popover shut. `dropdown-portal` on the content root is a
				     `portalDivs` marker, so the enclosing Modal2's clickOutside treats the whole
				     popover surface — padding included — as inside a portal and stays open. -->
				<Popover
					floatingConfig={{ strategy: 'fixed', placement: 'bottom-end' }}
					portal="body"
					contentClasses="p-4 dropdown-portal"
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
					portal="body"
					contentClasses="p-4 dropdown-portal"
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
	{/snippet}

	<CriticalAlertModalInner
		bind:workspaceContext
		{muteSettings}
		{numUnacknowledgedCriticalAlerts}
		{updateHasUnacknowledgedCriticalAlerts}
		{getCriticalAlerts}
		{acknowledgeCriticalAlert}
		{acknowledgeAllCriticalAlerts}
		bind:this={childRef}
	/>
</Modal2>
