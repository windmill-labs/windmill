<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import InstanceSettings from '$lib/components/InstanceSettings.svelte'
	import { Alert, Button } from '$lib/components/common'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'
	import {
		setupNavigationGroups,
		tabToCategoryMap,
		tabToAuthSubTab,
		categoryToTabMap,
		buildSearchableSettingItems,
		type SearchableSettingItem
	} from '$lib/components/instanceSettings'
	import SettingsSearchInput from '$lib/components/instanceSettings/SettingsSearchInput.svelte'
	import Breadcrumb from '$lib/components/common/breadcrumb/Breadcrumb.svelte'
	import { ChevronRight, ArrowLeft } from 'lucide-svelte'
	import { superadmin } from '$lib/stores'
	import { onDestroy, tick } from 'svelte'
	import { UserService, JobService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import SettingCard from '$lib/components/instanceSettings/SettingCard.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	const settingsSteps = [
		{ id: 'Core', label: 'Core' },
		{ id: 'Auth/OAuth/SAML', label: 'Authentication' }
	] as const

	const wizardStepLabels = [...settingsSteps.map((s) => s.label), 'Root login & Resource Types']

	const fullStepLabels = ['Settings', 'Root login & Resource Types']

	const initialMode = $page.url.searchParams.get('mode') === 'full' ? 'full' : 'wizard'
	const initialStep = Math.max(
		0,
		Math.min(parseInt($page.url.searchParams.get('step') ?? '0') || 0, wizardStepLabels.length - 1)
	)
	const initialFullStep =
		initialMode === 'full'
			? Math.max(
					0,
					Math.min(
						parseInt($page.url.searchParams.get('step') ?? '0') || 0,
						fullStepLabels.length - 1
					)
				)
			: 0
	let mode: 'wizard' | 'full' = $state(initialMode)
	let wizardStep = $state(initialStep)
	let fullStep = $state(initialFullStep)

	$effect(() => {
		const url = new URL(window.location.href)
		if (mode === 'wizard') {
			url.searchParams.set('step', String(wizardStep))
			url.searchParams.delete('mode')
		} else {
			url.searchParams.set('step', String(fullStep))
			url.searchParams.set('mode', 'full')
		}
		history.replaceState(history.state, '', url)
	})

	let instanceSettings: InstanceSettings | undefined = $state()

	function isSettingsStep(step: number): boolean {
		return step < settingsSteps.length
	}

	let currentStepDirty = $derived(
		isSettingsStep(wizardStep)
			? (instanceSettings?.isDirty(settingsSteps[wizardStep].id) ?? false)
			: false
	)

	// --- Account step state ---
	let newEmail = $state('')
	let newPassword = $state('')
	let enableHubSync = $state(true)
	let accountSubmitting = $state(false)
	let accountError = $state('')

	// --- Resource type sync (triggered on entering account step) ---
	let rtSyncStatus: 'idle' | 'loading' | 'success' | 'error' = $state('idle')
	let rtSyncMessage = $state('')

	async function syncCachedResourceTypes() {
		rtSyncStatus = 'loading'
		rtSyncMessage = ''
		try {
			const res = await fetch('/api/settings/sync_cached_resource_types', { method: 'POST' })
			if (!res.ok) {
				const body = await res.text()
				throw new Error(body || res.statusText)
			}
			rtSyncMessage = await res.text()
			rtSyncStatus = 'success'
		} catch (e: any) {
			rtSyncMessage = e?.message ?? 'Failed to sync resource types'
			rtSyncStatus = 'error'
		}
	}

	$effect(() => {
		if (
			rtSyncStatus === 'idle' &&
			((mode === 'wizard' && !isSettingsStep(wizardStep)) || (mode === 'full' && fullStep === 1))
		) {
			syncCachedResourceTypes()
		}
	})

	// --- Live hub sync ---
	let hubSyncStatus: 'idle' | 'loading' | 'success' | 'error' = $state('idle')
	let hubSyncMessage = $state('')

	async function syncFromHub() {
		hubSyncStatus = 'loading'
		hubSyncMessage = ''
		try {
			await JobService.runWaitResultScriptByPath({
				workspace: 'admins',
				path: 'u/admin/hub_sync',
				requestBody: {}
			})
			hubSyncStatus = 'success'
			hubSyncMessage = 'Resource types synced from hub successfully'
		} catch (e: any) {
			hubSyncMessage =
				e?.body?.error?.message ||
				e?.body?.message ||
				(typeof e?.body === 'string' ? e.body : null) ||
				e?.message ||
				'Failed to sync from hub'
			hubSyncStatus = 'error'
		}
	}

	const emailPattern = /^[\w-.]+@([\w-]+\.)+[\w-]{2,4}$/
	let emailValid = $derived(emailPattern.test(newEmail))
	let passwordValid = $derived(newPassword.length >= 2)
	let accountFormValid = $derived(emailValid && passwordValid)

	// --- EE license key warning ---
	let showLicenseKeyWarning = $state(false)
	let pendingNextCallback: (() => void) | undefined = $state(undefined)

	function isEeImage(): boolean {
		const v = instanceSettings?.getVersion() ?? ''
		return v.startsWith('EE')
	}

	function isLicenseKeyEmpty(): boolean {
		const key = instanceSettings?.getLicenseKey() ?? ''
		return key.trim() === ''
	}

	// --- Full settings mode state ---
	let fullTab = $state('general')
	let instanceSettingsCategory = $derived(tabToCategoryMap[fullTab] ?? 'Core')
	let authSubTab: 'sso' | 'oauth' | 'scim' = $derived(tabToAuthSubTab[fullTab] ?? 'sso')
	let yamlMode = $state(false)

	function handleNavigate(newTab: string) {
		if (newTab === fullTab) return
		fullTab = newTab
	}

	// --- Settings search (full mode) ---
	const searchableItems = buildSearchableSettingItems(setupNavigationGroups)

	let scrollTimeout: ReturnType<typeof setTimeout> | undefined
	let highlightTimeout: ReturnType<typeof setTimeout> | undefined

	async function handleSearchSelect(item: SearchableSettingItem) {
		handleNavigate(item.tabId)
		if (item.settingKey) {
			clearTimeout(scrollTimeout)
			clearTimeout(highlightTimeout)
			await tick()
			scrollTimeout = setTimeout(() => {
				const el = document.querySelector(`[data-setting-key="${item.settingKey}"]`)
				if (el) {
					el.scrollIntoView({ behavior: 'smooth', block: 'center' })
					el.classList.add('setting-highlight')
					highlightTimeout = setTimeout(() => el.classList.remove('setting-highlight'), 2500)
				}
			}, 100)
		}
	}

	onDestroy(() => {
		clearTimeout(scrollTimeout)
		clearTimeout(highlightTimeout)
	})

	/** Check if we need to warn about missing EE license key before proceeding */
	function proceedFromCore(callback: () => void) {
		const leavingSettings =
			(mode === 'wizard' && wizardStep === 0) || (mode === 'full' && fullStep === 0)
		if (leavingSettings && isEeImage() && isLicenseKeyEmpty()) {
			pendingNextCallback = callback
			showLicenseKeyWarning = true
			return
		}
		saveAndProceed(callback)
	}

	/** Auto-save dirty settings, then run the callback */
	async function saveAndProceed(callback: () => void) {
		if (yamlMode) {
			// In YAML mode, sync editor → form, then bulk-save everything
			if (!instanceSettings?.syncBeforeDiff()) return
			await instanceSettings.saveSettings()
		} else if (mode === 'full') {
			// In full (advanced) mode, bulk-save all settings
			await instanceSettings?.saveSettings()
		} else if (isSettingsStep(wizardStep)) {
			const category = settingsSteps[wizardStep].id
			if (instanceSettings?.isDirty(category)) {
				await instanceSettings.saveCategorySettings(category)
			}
		}
		callback()
	}

	function switchToFullMode() {
		mode = 'full'
	}

	function switchToWizardMode() {
		yamlMode = false
		fullStep = 0
		mode = 'wizard'
	}

	function finishSetup() {
		goto('/user/workspaces')
	}

	async function submitAccount() {
		accountError = ''
		accountSubmitting = true
		try {
			let oldEmail = $superadmin
			if (!oldEmail) {
				oldEmail = await UserService.getCurrentEmail()
			}
			if (!oldEmail) {
				throw new Error('Could not determine current admin email')
			}

			await UserService.createUserGlobally({
				requestBody: {
					email: newEmail,
					password: newPassword,
					super_admin: true
				}
			})

			const token = await UserService.login({
				requestBody: { email: newEmail, password: newPassword }
			})

			// Update the client token for subsequent requests
			const { OpenAPI } = await import('$lib/gen')
			OpenAPI.TOKEN = token

			if (enableHubSync) {
				try {
					// Use direct fetch with token as query param to avoid the old session cookie
					// overriding the Authorization header
					const resp = await fetch(
						`/api/w/admins/schedules/create?token=${encodeURIComponent(token)}`,
						{
							method: 'POST',
							headers: { 'Content-Type': 'application/json' },
							body: JSON.stringify({
								path: 'g/all/hub_sync',
								schedule: '0 0 0 * * *',
								script_path: 'u/admin/hub_sync',
								is_flow: false,
								args: {},
								enabled: true,
								timezone: 'Etc/UTC'
							})
						}
					)
					if (!resp.ok) {
						console.warn('Schedule creation failed:', await resp.text())
					}
				} catch (e: any) {
					console.warn('Schedule creation failed:', e?.body ?? e)
				}
			}

			try {
				await UserService.globalUserDelete({ email: oldEmail })
			} catch (e: any) {
				console.warn('Deleting old account failed:', e?.body ?? e)
			}

			sendUserToast('Account setup complete')
			goto(
				'/user/logout?rd=' +
					encodeURIComponent(
						'/user/login?email=' +
							encodeURIComponent(newEmail) +
							'&password=' +
							encodeURIComponent(newPassword)
					)
			)
		} catch (e: any) {
			accountError = e?.body?.message || e?.body || e?.message || 'An error occurred'
		} finally {
			accountSubmitting = false
		}
	}
</script>

{#snippet accountSetupContent()}
	<SettingsPageHeader title="Root login & Resource Types" />

	<div class="flex flex-col gap-6 pb-6">
		<SettingCard
			label="Superadmin login"
			description="Replace the default superadmin account with a secure email and password."
		>
			<div class="flex flex-col gap-4">
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-secondary">Email</span>
					<TextInput
						bind:value={newEmail}
						inputProps={{ type: 'email', placeholder: 'admin@company.com' }}
						error={newEmail.length > 0 && !emailValid ? 'Must be a valid email' : undefined}
						size="md"
					/>
					{#if $superadmin}
						<p class="text-tertiary text-2xs mt-1">Current email: {$superadmin}</p>
					{/if}
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-secondary">Password</span>
					<TextInput
						bind:value={newPassword}
						inputProps={{ type: 'password', placeholder: 'Enter password' }}
						error={newPassword.length > 0 && !passwordValid
							? 'Must be at least 2 characters'
							: undefined}
						size="md"
					/>
				</div>
			</div>
		</SettingCard>

		<SettingCard
			label="Resource Types"
			description="Resource types bundled with the Docker image are synced automatically. You can also fetch the latest from the hub."
		>
			<div class="flex flex-col gap-3 mt-1">
				{#if rtSyncStatus === 'loading'}
					<Alert type="info" title="Syncing cached resource types..." />
				{:else if rtSyncStatus === 'success'}
					<Alert type="success" title="Cached resource types synced">
						{rtSyncMessage}
					</Alert>
				{:else if rtSyncStatus === 'error'}
					<Alert type="error" title="Cached resource types sync failed">
						{rtSyncMessage}
					</Alert>
				{/if}

				<div class="flex items-center gap-2">
					<Button
						variant="accent"
						unifiedSize="sm"
						loading={hubSyncStatus === 'loading'}
						onClick={syncFromHub}
					>
						Sync latest from hub
					</Button>
					<p class="text-tertiary text-2xs">
						Fetches the latest resource types directly from the Windmill Hub (requires internet
						access).
					</p>
				</div>
				{#if hubSyncStatus === 'success'}
					<Alert type="success" title="Hub sync complete">
						{hubSyncMessage}
					</Alert>
				{:else if hubSyncStatus === 'error'}
					<Alert type="error" title="Hub sync failed">
						{hubSyncMessage}
					</Alert>
				{/if}
				<Toggle
					bind:checked={enableHubSync}
					options={{ right: 'Sync resource types every day' }}
					size="xs"
				/>
				<p class="text-tertiary text-2xs">
					The daily schedule synchronizes resource types from the Hub every day at midnight UTC.
				</p>
			</div>
		</SettingCard>

		{#if accountError}
			<Alert type="error" title="Setup error">
				{accountError}
			</Alert>
		{/if}
	</div>
{/snippet}

<CenteredModal large title="Instance settings" centerVertically={false} containOverflow>
	<div class="flex flex-col flex-1 min-h-0 overflow-hidden">
		{#if mode === 'wizard'}
			<!-- Step indicator (pinned top) -->
			<div class="pb-2 border-b shrink-0 flex justify-start">
				<Breadcrumb
					items={wizardStepLabels}
					selectedIndex={wizardStep + 1}
					numbered
					onselect={(i) => {
						if (i < wizardStep) saveAndProceed(() => (wizardStep = i))
					}}
				>
					{#snippet separator()}
						<ChevronRight size={16} class="text-tertiary shrink-0" />
					{/snippet}
				</Breadcrumb>
			</div>

			<!-- Step content (scrollable) -->
			<div class="flex-1 overflow-auto min-h-0 pt-4">
				{#if isSettingsStep(wizardStep)}
					{#if settingsSteps[wizardStep].id === 'Auth/OAuth/SAML'}
						<p class="text-secondary text-xs mb-4">
							Windmill uses its own authentication by default. SSO configuration is optional and can
							be set up later.
						</p>
					{/if}
					{#key wizardStep}
						<InstanceSettings
							bind:this={instanceSettings}
							hideTabs
							quickSetup
							tab={settingsSteps[wizardStep].id}
						/>
					{/key}
				{:else}
					{@render accountSetupContent()}
				{/if}
			</div>
		{:else}
			<!-- Breadcrumb (full mode) -->
			<div class="pb-2 border-b shrink-0 flex items-center justify-between">
				<Breadcrumb
					items={fullStepLabels}
					selectedIndex={fullStep + 1}
					numbered
					onselect={(i) => {
						if (i < fullStep) {
							saveAndProceed(() => {
								yamlMode = false
								fullStep = i
							})
						}
					}}
				>
					{#snippet separator()}
						<ChevronRight size={16} class="text-tertiary shrink-0" />
					{/snippet}
				</Breadcrumb>
				{#if fullStep === 0}
					<Toggle bind:checked={yamlMode} options={{ right: 'YAML' }} size="sm" />
				{/if}
			</div>

			<!-- Full mode content -->
			{#if fullStep === 0}
				<div class="flex flex-1 min-h-0 pt-4">
					{#if !yamlMode}
						<div class="w-44 shrink-0 h-full overflow-auto pb-4 pr-4">
							<SettingsSearchInput {searchableItems} onSelect={handleSearchSelect} class="mb-3" />
							<SidebarNavigation
								groups={setupNavigationGroups}
								selectedId={fullTab}
								onNavigate={handleNavigate}
							/>
						</div>
					{/if}

					<div class="flex-1 min-w-0 h-full overflow-auto px-4">
						<InstanceSettings
							bind:this={instanceSettings}
							hideTabs
							tab={instanceSettingsCategory}
							{authSubTab}
							bind:yamlMode
							onNavigateToTab={(category) => {
								const targetTab = categoryToTabMap[category]
								if (targetTab) {
									handleNavigate(targetTab)
								}
							}}
						/>
					</div>
				</div>
			{:else}
				<div class="flex-1 overflow-auto min-h-0 pt-4">
					{@render accountSetupContent()}
				</div>
			{/if}
		{/if}

		<!-- Navigation (pinned bottom) -->
		<div class="flex items-center justify-between pt-4 border-t shrink-0">
			{#if mode === 'wizard'}
				<div class="flex items-center gap-2">
					{#if wizardStep > 0}
						<Button
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: ArrowLeft }}
							onClick={() => saveAndProceed(() => (wizardStep -= 1))}
						>
							Back
						</Button>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					{#if isSettingsStep(wizardStep)}
						<Button
							variant="default"
							unifiedSize="md"
							onClick={() => saveAndProceed(switchToFullMode)}
						>
							Advanced setup
						</Button>
					{/if}
					{#if wizardStep < wizardStepLabels.length - 1}
						<Button
							variant="accent"
							unifiedSize="md"
							onClick={() => proceedFromCore(() => (wizardStep += 1))}
						>
							{currentStepDirty ? 'Save & Next' : 'Next'}
						</Button>
					{:else}
						<Button
							variant="accent"
							unifiedSize="md"
							disabled={!accountFormValid}
							loading={accountSubmitting}
							onClick={submitAccount}
						>
							Set account & finish
						</Button>
					{/if}
				</div>
			{:else if fullStep === 0}
				<Button
					variant="default"
					unifiedSize="md"
					startIcon={{ icon: ArrowLeft }}
					onClick={switchToWizardMode}
				>
					Quick setup
				</Button>
				<Button
					variant="accent"
					unifiedSize="md"
					onClick={() =>
						proceedFromCore(() => {
							yamlMode = false
							fullStep = 1
						})}
				>
					Continue
				</Button>
			{:else}
				<Button
					variant="default"
					unifiedSize="md"
					startIcon={{ icon: ArrowLeft }}
					onClick={() => saveAndProceed(() => (fullStep = 0))}
				>
					Back
				</Button>
				<Button
					variant="accent"
					unifiedSize="md"
					disabled={!accountFormValid}
					loading={accountSubmitting}
					onClick={submitAccount}
				>
					Set account & finish
				</Button>
			{/if}
		</div>

		<div class="flex items-center justify-start gap-2 mt-2 shrink-0">
			<p class="text-secondary text-xs">
				You can change these settings later in the instance settings.
			</p>
			<Button variant="subtle" unifiedSize="sm" onClick={finishSetup}>Skip setup</Button>
		</div>
	</div>
</CenteredModal>

{#if showLicenseKeyWarning}
	<ConfirmationModal
		open={showLicenseKeyWarning}
		title="License key required"
		confirmationText="Continue without license key"
		on:canceled={() => {
			showLicenseKeyWarning = false
			pendingNextCallback = undefined
		}}
		on:confirmed={() => {
			showLicenseKeyWarning = false
			const cb = pendingNextCallback
			pendingNextCallback = undefined
			if (cb) saveAndProceed(cb)
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>
				You are running the Enterprise Edition image but have not entered a license key. A valid
				license key is required to use EE features. Are you sure you want to continue without one?
			</span>
		</div>
	</ConfirmationModal>
{/if}
