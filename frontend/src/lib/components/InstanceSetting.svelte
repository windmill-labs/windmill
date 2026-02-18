<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense, isCriticalAlertsUIOpen } from '$lib/stores'
	import { AlertCircle, BadgeCheck, BadgeX, Info } from 'lucide-svelte'
	import type { Setting } from './instanceSettings'
	import { OTEL_TRACING_PROXY_LANGUAGES } from './instanceSettings'
	import { LanguageIcon } from './common/languageIcons'
	import ObjectStoreConfigSettings from './ObjectStoreConfigSettings.svelte'
	import { sendUserToast } from '$lib/toast'
	import { ConfigService, SettingService, type ListAvailablePythonVersionsResponse } from '$lib/gen'
	import { Button, SecondsInput, Skeleton } from './common'
	import Password from './Password.svelte'
	import { classNames } from '$lib/utils'
	import Popover from './Popover.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import Toggle from './Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import { createEventDispatcher, untrack } from 'svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import CriticalAlertChannels from './instanceSettings/CriticalAlertChannels.svelte'
	import SmtpSettings from './instanceSettings/SmtpSettings.svelte'
	import SecretBackendConfig from './instanceSettings/SecretBackendConfig.svelte'
	import IndexerMemorySettings from './instanceSettings/IndexerMemorySettings.svelte'
	import IndexerJobIndexSettings from './instanceSettings/IndexerJobIndexSettings.svelte'
	import IndexerLogIndexSettings from './instanceSettings/IndexerLogIndexSettings.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'

	interface Props {
		setting: Setting
		version: string
		values: Writable<Record<string, any>>
		loading?: boolean
		openSmtpSettings?: () => void
		oauths?: Record<string, any>
		warning?: string
	}

	let {
		setting,
		version,
		values,
		loading = true,
		openSmtpSettings,
		oauths,
		warning
	}: Props = $props()
	const dispatch = createEventDispatcher()

	let latestKeyRenewalAttempt: {
		result: string
		attempted_at: string
	} | null = $state(null)

	function showSetting(setting: string, values: Record<string, any>) {
		if (setting == 'dev_instance') {
			if (values['license_key'] == undefined) {
				return false
			}
		}
		return true
	}

	let renewing = $state(false)
	let opening = $state(false)

	async function reloadKeyrenewalAttemptInfo() {
		latestKeyRenewalAttempt = await SettingService.getLatestKeyRenewalAttempt()
	}

	async function reloadLicenseKey() {
		$values['license_key'] = await SettingService.getGlobal({
			key: 'license_key'
		})
	}

	$effect(() => {
		if (setting.key == 'license_key') {
			untrack(() => reloadKeyrenewalAttemptInfo())
		}
	})

	export async function renewLicenseKey() {
		renewing = true
		try {
			await SettingService.renewLicenseKey({
				licenseKey: $values['license_key'] || undefined
			})
			sendUserToast('Key renewal successful')
			reloadLicenseKey()
		} catch (err) {
			throw err
		} finally {
			reloadKeyrenewalAttemptInfo()
			renewing = false
		}
	}

	export async function openCustomerPortal() {
		opening = true
		try {
			const url = await SettingService.createCustomerPortalSession({
				licenseKey: $values['license_key'] || undefined
			})
			window.open(url, '_blank')
		} finally {
			opening = false
		}
	}

	function parseLicenseKey(key: string): {
		valid: boolean
		expiration?: string
	} {
		let splitted = key.split('.')
		if (splitted.length >= 3) {
			try {
				let i = parseInt(splitted[1])
				let date = new Date(i * 1000)
				const stringDate = date.toLocaleDateString()
				if (stringDate !== 'Invalid Date') {
					return {
						valid: date.getTime() > Date.now(),
						expiration: date.toLocaleDateString()
					}
				}
			} catch {}
		}
		return {
			valid: false
		}
	}

	$effect(() => {
		if (setting.key === 'license_key') {
			const key = $values['license_key'] ?? ''
			const { valid } = parseLicenseKey(key)
			if (valid) {
				$enterpriseLicense = key.split('.')[0]
			}
		}
	})

	let pythonAvailableVersions: ListAvailablePythonVersionsResponse = $state([])

	let isPyFetching = $state(false)
	async function fetch_available_python_versions() {
		if (isPyFetching) return
		isPyFetching = true
		try {
			pythonAvailableVersions = await ConfigService.listAvailablePythonVersions()
		} catch (error) {
			console.error('Error fetching python versions:', error)
		} finally {
			isPyFetching = false
		}
	}

	$effect(() => {
		if (setting.fieldType == 'select_python') {
			untrack(() => fetch_available_python_versions())
		}
	})

	$effect(() => {
		if (
			(setting.fieldType == 'select' || setting.fieldType == 'select_python') &&
			$values[setting.key] == undefined &&
			setting.defaultValue
		) {
			untrack(() => {
				if (setting.defaultValue) {
					$values[setting.key] = setting.defaultValue()
				}
			})
		}
	})
</script>

<!-- {JSON.stringify($values, null, 2)} -->
{#if (!setting.cloudonly || isCloudHosted()) && showSetting(setting.key, $values) && !(setting.hiddenIfNull && $values[setting.key] == null) && !(setting.hiddenIfEmpty && !$values[setting.key]) && !(setting.hiddenInEe && $enterpriseLicense)}
	{#if setting.fieldType == 'select'}
		<SettingCard
			label={setting.label}
			description={setting.description}
			ee_only={setting.ee_only}
			settingKey={setting.key}
		>
			<ToggleButtonGroup bind:selected={$values[setting.key]}>
				{#snippet children({ item: toggleButton })}
					{#each setting.select_items ?? [] as item}
						<ToggleButton
							value={item.value ?? item.label}
							label={item.label}
							tooltip={item.tooltip}
							item={toggleButton}
						/>
					{/each}
				{/snippet}
			</ToggleButtonGroup>
		</SettingCard>
	{:else if setting.fieldType == 'select_python'}
		<SettingCard
			label={setting.label}
			description={setting.description}
			ee_only={setting.ee_only}
			settingKey={setting.key}
		>
			<ToggleButtonGroup bind:selected={$values[setting.key]}>
				{#snippet children({ item: toggleButton })}
					{#each setting.select_items ?? [] as item}
						<ToggleButton
							value={item.value ?? item.label}
							label={item.label}
							tooltip={item.tooltip}
							item={toggleButton}
						/>
					{/each}

					<DropdownV2
						items={() =>
							pythonAvailableVersions.map((v) => ({
								displayName: v,
								action: () => {
									$values[setting.key] = v
								}
							}))}
					>
						{#snippet buttonReplacement()}
							{#if setting.select_items?.some((e) => e.label == $values[setting.key] || e.value == $values[setting.key])}
								<Button variant="subtle" btnClasses="font-normal" nonCaptureEvent={true}
									>Select Custom</Button
								>
							{:else}
								<Button
									variant="default"
									btnClasses="font-normal bg-surface-input"
									nonCaptureEvent={true}>Custom | {$values[setting.key]}</Button
								>
							{/if}
						{/snippet}
					</DropdownV2>
				{/snippet}
			</ToggleButtonGroup>
		</SettingCard>
	{:else if setting.fieldType == 'indexer_rates'}
		{#if $values[setting.key]}
			{@const fieldErrors = setting.validate?.($values[setting.key]) ?? {}}
			<SettingCard
				label="Memory"
				description="Configure the memory budget for the indexer and manage index clearing."
				ee_only=""
				settingKey="indexer_settings_memory"
			>
				<div class="p-4 rounded-md border mt-2">
					<IndexerMemorySettings {values} disabled={!$enterpriseLicense} errors={fieldErrors} />
				</div>
			</SettingCard>
			<SettingCard
				label="Completed Job Index"
				description="Configure indexing parameters for completed jobs."
				ee_only=""
				settingKey="indexer_settings_jobs"
			>
				<div class="p-4 rounded-md border mt-2">
					<IndexerJobIndexSettings {values} disabled={!$enterpriseLicense} errors={fieldErrors} />
				</div>
			</SettingCard>
			<SettingCard
				label="Service Logs Index"
				description="Configure indexing parameters for service logs."
				ee_only=""
				settingKey="indexer_settings_logs"
			>
				<div class="p-4 rounded-md border mt-2">
					<IndexerLogIndexSettings {values} disabled={!$enterpriseLicense} errors={fieldErrors} />
				</div>
			</SettingCard>
		{/if}
	{:else}
		<SettingCard
			label={setting.fieldType != 'smtp_connect' ? setting.label : undefined}
			description={setting.description}
			ee_only={setting.ee_only}
			tooltip={setting.tooltip}
			settingKey={setting.key}
			actionButton={setting.actionButton}
			values={$values}
		>
			{#if $values}
				{@const hasError = setting.isValid && !setting.isValid($values[setting.key])}
				<div class="h-1"></div>
				{#if loading}
					<Skeleton layout={[[2.5]]} />
				{:else if setting.fieldType == 'text'}
					<TextInput
						inputProps={{
							type: 'text',
							id: setting.key,
							disabled: setting.ee_only != undefined && !$enterpriseLicense,
							placeholder: setting.placeholder
						}}
						bind:value={$values[setting.key]}
						class="max-w-lg"
					/>
					{#if warning}
						<span class="text-yellow-600 dark:text-yellow-500 text-2xs">
							{warning}
						</span>
					{/if}
					{#if setting.advancedToggle}
						<div class="mt-1">
							<Toggle
								size="xs"
								options={{ right: setting.advancedToggle.label }}
								checked={setting.advancedToggle.checked($values)}
								on:change={() => {
									if (setting.advancedToggle) {
										$values = setting.advancedToggle.onChange($values)
									}
								}}
							/>
						</div>
					{/if}
				{:else if setting.fieldType == 'textarea'}
					<textarea
						id={setting.key}
						disabled={!$enterpriseLicense}
						rows="2"
						placeholder={setting.placeholder}
						bind:value={$values[setting.key]}
					></textarea>
					{#if setting.key == 'saml_metadata'}
						<div class="flex mt-2">
							<Button
								disabled={!$enterpriseLicense}
								on:click={async (e) => {
									try {
										const res = await SettingService.testMetadata({
											requestBody: $values[setting.key]
										})
										sendUserToast(`Metadata valid: ${res}`)
									} catch (error) {
										sendUserToast(`Invalid metadata`, true, error.message)
									}
								}}>Test content/url</Button
							>
						</div>
					{/if}
				{:else if setting.fieldType == 'codearea'}
					<SimpleEditor
						autoHeight
						class="editor"
						lang={setting.codeAreaLang ?? 'txt'}
						bind:code={$values[setting.key]}
						fixedOverflowWidgets={false}
					/>
				{:else if setting.fieldType == 'license_key'}
					{@const { valid, expiration } = parseLicenseKey($values[setting.key] ?? '')}
					<div class="flex gap-2">
						<Password
							id={setting.key}
							small
							placeholder={setting.placeholder}
							onBlur={() => {
								if ($values[setting.key] && typeof $values[setting.key] === 'string') {
									$values[setting.key] = $values[setting.key].trim()
								}
							}}
							bind:password={$values[setting.key]}
						/>
						<Button
							variant="default"
							unifiedSize="md"
							disabled={!$values[setting.key]}
							on:click={async () => {
								await SettingService.testLicenseKey({
									requestBody: { license_key: $values[setting.key] }
								})
								sendUserToast('Valid key')
							}}
						>
							Test key
						</Button>
					</div>
					<div class="mt-1 flex flex-col gap-1 items-start">
						{#if $values[setting.key]?.length > 0}
							{#if valid}
								<div class="flex flex-row gap-1 items-center">
									<Info size={12} class="text-primary" />
									<span class="text-primary text-xs">License key expires on {expiration ?? ''}</span
									>
								</div>
							{:else if expiration}
								<div class="flex flex-row gap-1 items-center">
									<AlertCircle size={12} class="text-red-600" />
									<span class="text-red-600 dark:text-red-400 text-xs">
										{#if $values[setting.key]?.endsWith('__dev')}
											Dev license key expired on {expiration}.<br />If even after successful
											renewal, your dev license key is still expired, it means that your production
											key has expired due to unpaid invoices or excessive use of your production
											instance.
										{:else}
											License key expired on {expiration}.
										{/if}
									</span>
								</div>
							{:else}
								<div class="flex flex-row gap-1 items-center">
									<AlertCircle size={12} class="text-red-600" />
									<span class="text-red-600 dark:text-red-400 text-xs"
										>Invalid license key format</span
									>
								</div>
							{/if}
						{/if}
						{#if latestKeyRenewalAttempt}
							{@const attemptedAt = new Date(latestKeyRenewalAttempt.attempted_at).toLocaleString()}
							{@const isTrial = latestKeyRenewalAttempt.result.startsWith('error: trial:')}
							<div class="relative">
								<Popover notClickable>
									<div class="flex flex-row items-center gap-1">
										{#if latestKeyRenewalAttempt.result === 'success'}
											<BadgeCheck class="text-green-600" size={12} />
										{:else}
											<BadgeX class={isTrial ? 'text-yellow-600' : 'text-red-600'} size={12} />
										{/if}
										<span
											class={classNames(
												'text-xs',
												latestKeyRenewalAttempt.result === 'success'
													? 'text-green-600'
													: isTrial
														? 'text-yellow-600'
														: 'text-red-600'
											)}
										>
											{#if latestKeyRenewalAttempt.result === 'success' && $values[setting.key]?.endsWith('__dev')}
												Latest dev key renewal succeeded on {attemptedAt}. The dev key expiry was
												updated to align with your current production key's expiration date.
											{:else}
												{latestKeyRenewalAttempt.result === 'success'
													? 'Latest key renewal succeeded'
													: isTrial
														? 'Latest key renewal ignored because in trial'
														: 'Latest key renewal failed'}
												on {attemptedAt}
											{/if}
										</span>
									</div>
									{#snippet text()}
										<div>
											{#if latestKeyRenewalAttempt?.result === 'success'}
												<span class="text-green-300">
													Latest key renewal succeeded on {attemptedAt}
												</span>
											{:else if isTrial}
												<span class="text-yellow-300">
													License key cannot be renewed during trial ({attemptedAt})
												</span>
											{:else}
												<span class="text-red-600 dark:text-red-400">
													Latest key renewal failed on {attemptedAt}: {latestKeyRenewalAttempt?.result.replace(
														'error: ',
														''
													)}
												</span>
											{/if}
											<br />
											As long as invoices are paid and usage corresponds to the subscription, the key
											is renewed daily with a validity of 35 days (grace period).
										</div>
									{/snippet}
								</Popover>
							</div>
						{/if}
						{#if $values[setting.key]?.length > 0 && version.includes('CE')}
							<div class="flex flex-row gap-1 items-center">
								<Info size={12} class="text-blue-600" />
								<span class="text-blue-600 dark:text-blue-400 text-xs">
									License key is set but the current image is Community Edition ({version}). Switch
									to the EE image to finalize the upgrade.
								</span>
							</div>
						{/if}

						{#if valid || expiration}
							<div class="flex flex-row gap-2 mt-1">
								<Button on:click={renewLicenseKey} loading={renewing} size="xs" variant="accent"
									>Renew key
								</Button>
								<Button variant="accent" size="xs" loading={opening} on:click={openCustomerPortal}>
									Open customer portal
								</Button>
							</div>
						{/if}
					</div>
				{:else if setting.fieldType == 'email'}
					<TextInput
						inputProps={{
							type: 'email',
							placeholder: setting.placeholder,
							id: setting.key
						}}
						bind:value={$values[setting.key]}
					/>
				{:else if setting.key == 'critical_alert_mute_ui'}
					<div class="flex flex-col gap-y-2">
						<Toggle
							disabled={!$enterpriseLicense}
							bind:checked={$values[setting.key]}
							id={setting.key}
						/>
						<div class="flex flex-row">
							<Button
								variant="default"
								disabled={!$enterpriseLicense}
								size="xs"
								on:click={() => {
									isCriticalAlertsUIOpen.set(true)
									dispatch('closeDrawer')
								}}
							>
								Show critical alerts
							</Button>
						</div>
					</div>
				{:else if setting.fieldType == 'critical_error_channels'}
					<CriticalAlertChannels {values} {openSmtpSettings} {oauths} />
				{:else if setting.fieldType == 'otel'}
					<div class="flex flex-col gap-4 p-4 rounded-md border">
						{#if $values[setting.key]}
							<div class="flex gap-8">
								<Toggle
									disabled={!$enterpriseLicense}
									id="tracing_enabled"
									bind:checked={$values[setting.key].tracing_enabled}
									options={{ right: 'Tracing' }}
								/>
								<Toggle
									disabled={!$enterpriseLicense}
									id="logs_enabled"
									bind:checked={$values[setting.key].logs_enabled}
									options={{ right: 'Logs' }}
								/>
								<Toggle
									disabled
									id="metrics_enabled"
									bind:checked={$values[setting.key].logs_enabled}
									options={{ right: 'Metrics (coming soon)' }}
								/>
							</div>

							<div class="flex flex-col gap-1">
								<label
									for="OTEL_EXPORTER_OTLP_ENDPOINT"
									class="block text-xs font-semibold text-emphasis">Endpoint</label
								>
								<TextInput
									inputProps={{
										type: 'text',
										placeholder: 'http://otel-collector.example.com:4317',
										id: 'OTEL_EXPORTER_OTLP_ENDPOINT',
										disabled: !$enterpriseLicense
									}}
									bind:value={$values[setting.key].otel_exporter_otlp_endpoint}
								/>
							</div>
							<div class="flex flex-col gap-1">
								<label
									for="OTEL_EXPORTER_OTLP_HEADERS"
									class="block text-xs font-semibold text-emphasis">Headers</label
								>
								<TextInput
									inputProps={{
										type: 'text',
										placeholder: 'Authorization=Bearer my-secret-token,Env=production',
										id: 'OTEL_EXPORTER_OTLP_HEADERS',
										disabled: !$enterpriseLicense
									}}
									bind:value={$values[setting.key].otel_exporter_otlp_headers}
								/>
							</div>
							<div class="flex flex-col gap-1">
								<label
									for="OTEL_EXPORTER_OTLP_PROTOCOL"
									class="block text-xs font-semibold text-emphasis">Protocol</label
								>
								<span class="text-primary font-normal text-xs">gRPC</span>
							</div>
							<!-- <div>
							<label for="OTEL_EXPORTER_OTLP_PROTOCOL" class="block text-sm font-semibold"
								>Protocol<span class="text-2xs text-primary ml-4"
									>grpc, http/protobuf, http/json</span
								></label
							>
							<input
								type="text"
								id="OTEL_EXPORTER_OTLP_PROTOCOL"
								placeholder="grpc"
								bind:value={$values[setting.key].otel_exporter_otlp_protocol}
							/>
						</div>
						<div>
							<label for="OTEL_EXPORTER_OTLP_COMPRESSION" class="block text-sm font-semibold"
								>Compression <span class="text-2xs text-primary ml-4">none, gzip</span></label
							>
							<input
								type="text"
								id="OTEL_EXPORTER_OTLP_COMPRESSION"
								placeholder="none"
								bind:value={$values[setting.key].otel_exporter_otlp_compression}
							/>
						</div> -->
						{/if}
					</div>
				{:else if setting.fieldType == 'otel_tracing_proxy'}
					{@const tracingProxyVal = $values[setting.key] ?? {
						enabled: false,
						enabled_languages: [...OTEL_TRACING_PROXY_LANGUAGES]
					}}
					<div class="flex flex-col gap-4">
						<Toggle
							id="otel_tracing_proxy_enabled"
							checked={tracingProxyVal.enabled ?? false}
							on:change={(e) => {
								$values[setting.key] = { ...tracingProxyVal, enabled: e.detail }
							}}
							options={{ right: 'Enabled' }}
						/>
						{#if tracingProxyVal.enabled}
							<div class="flex flex-wrap gap-2">
								{#each OTEL_TRACING_PROXY_LANGUAGES as lang (lang)}
									{@const isEnabled = (tracingProxyVal.enabled_languages ?? []).includes(lang)}
									<button
										class="flex flex-col items-center gap-1 p-2 rounded border transition-all {isEnabled
											? 'border-blue-500 bg-blue-500/10'
											: 'border-gray-300 opacity-40 hover:opacity-70'}"
										onclick={() => {
											const current = tracingProxyVal.enabled_languages ?? []
											const newLangs = isEnabled
												? current.filter((l) => l !== lang)
												: [...current, lang]
											$values[setting.key] = { ...tracingProxyVal, enabled_languages: newLangs }
										}}
									>
										<LanguageIcon {lang} size={24} />
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{:else if setting.fieldType == 'object_store_config'}
					<ObjectStoreConfigSettings bind:bucket_config={$values[setting.key]} />
				{:else if setting.fieldType == 'critical_alerts_on_db_oversize'}
					{#if $values[setting.key]}
						<div class="flex flex-row flex-wrap gap-2 p-0 items-center">
							<div class="p-1">
								<Toggle
									disabled={!$enterpriseLicense}
									bind:checked={$values[setting.key].enabled}
									id={setting.key}
								/>
							</div>
							{#if $values[setting.key].enabled}
								<label class="block shrink min-w-0">
									<input
										type="number"
										placeholder={setting.placeholder}
										bind:value={$values[setting.key].value}
									/>
								</label>
								<span class="text-primary font-semibold text-sm">GB</span>
							{/if}
						</div>
					{/if}
				{:else if setting.fieldType == 'number'}
					<TextInput
						inputProps={{
							type: 'number',
							placeholder: setting.placeholder,
							id: setting.key
						}}
						bind:value={$values[setting.key]}
						class="max-w-lg"
					/>
				{:else if setting.fieldType == 'password'}
					<Password small placeholder={setting.placeholder} bind:password={$values[setting.key]} />
				{:else if setting.fieldType == 'boolean'}
					<Toggle
						disabled={setting.ee_only != undefined && !$enterpriseLicense}
						bind:checked={$values[setting.key]}
						id={setting.key}
					/>
				{:else if setting.fieldType == 'seconds'}
					<div>
						<SecondsInput
							max={setting.ee_only != undefined && !$enterpriseLicense
								? 60 * 60 * 24 * 30
								: undefined}
							bind:seconds={$values[setting.key]}
							clearable
						/>
					</div>
				{:else if setting.fieldType == 'smtp_connect'}
					<SmtpSettings {values} disabled={loading} />
				{:else if setting.fieldType == 'secret_backend'}
					<SecretBackendConfig {values} disabled={loading} />
				{/if}
				{#if hasError}
					<span class="text-red-600 dark:text-red-400 text-xs">
						{setting.error ?? ''}
					</span>
				{/if}
			{:else}
				<input disabled placeholder="Loading..." />
			{/if}
		</SettingCard>
	{/if}
{/if}
