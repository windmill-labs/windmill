<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense, isCriticalAlertsUIOpen } from '$lib/stores'
	import { AlertCircle, BadgeCheck, BadgeX, Info } from 'lucide-svelte'
	import type { Setting } from './instanceSettings'
	import Tooltip from './Tooltip.svelte'
	import ObjectStoreConfigSettings from './ObjectStoreConfigSettings.svelte'
	import { sendUserToast } from '$lib/toast'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import {
		ConfigService,
		IndexSearchService,
		SettingService,
		type ListAvailablePythonVersionsResponse
	} from '$lib/gen'
	import { Button, SecondsInput, Section, Skeleton } from './common'
	import Password from './Password.svelte'
	import { classNames } from '$lib/utils'
	import Popover from './Popover.svelte'
	import PopoverMelt from './meltComponents/Popover.svelte'
	import Toggle from './Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import LoadingIcon from './apps/svelte-select/lib/LoadingIcon.svelte'
	import EEOnly from './EEOnly.svelte'
	import CriticalAlertChannels from './instanceSettings/CriticalAlertChannels.svelte'
	import SmtpSettings from './instanceSettings/SmtpSettings.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Label from './Label.svelte'

	interface Props {
		setting: Setting
		version: string
		values: Writable<Record<string, any>>
		loading?: boolean
		openSmtpSettings?: () => void
		oauths?: Record<string, any>
	}

	let { setting, version, values, loading = true, openSmtpSettings, oauths }: Props = $props()
	const dispatch = createEventDispatcher()

	if (
		(setting.fieldType == 'select' || setting.fieldType == 'select_python') &&
		$values[setting.key] == undefined
	) {
		$values[setting.key] = 'default'
	}

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

	let licenseKeyChanged = $state(false)
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

	if (setting.key == 'license_key') {
		reloadKeyrenewalAttemptInfo()
	}

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

	let pythonAvailableVersions: ListAvailablePythonVersionsResponse = $state([])

	let isPyFetching = $state(false)
	let clearJobsIndexModalOpen = $state(false)
	let clearServiceLogsIndexModalOpen = $state(false)
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
	if (setting.fieldType == 'select_python') {
		fetch_available_python_versions()
	}
</script>

{#snippet LabelSnippet()}
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="flex flex-col gap-1 mb-1">
		<div class="flex gap-1">
			<span class="text-emphasis font-semibold text-xs">{setting.label}</span>
			{#if setting.ee_only != undefined && !$enterpriseLicense}
				<EEOnly>
					{#if setting.ee_only != ''}{setting.ee_only}{/if}
				</EEOnly>
			{/if}
		</div>
		{#if setting.description}
			<span class="text-secondary text-xs font-normal">
				{@html setting.description}
			</span>
		{/if}
	</label>
{/snippet}

<!-- {JSON.stringify($values, null, 2)} -->
{#if (!setting.cloudonly || isCloudHosted()) && showSetting(setting.key, $values) && !(setting.hiddenIfNull && $values[setting.key] == null) && !(setting.hiddenIfEmpty && !$values[setting.key])}
	{#if setting.fieldType == 'select'}
		<div>
			{@render LabelSnippet()}
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
		</div>
	{:else if setting.fieldType == 'select_python'}
		<div>
			<!-- svelte-ignore a11y_label_has_associated_control -->
			{@render LabelSnippet()}

			<ToggleButtonGroup bind:selected={$values[setting.key]}>
				{#snippet children({ item: toggleButtonn })}
					{#each setting.select_items ?? [] as item}
						<ToggleButton
							value={item.value ?? item.label}
							label={item.label}
							tooltip={item.tooltip}
							item={toggleButtonn}
						/>
					{/each}
					<PopoverMelt closeButton={!isPyFetching} contentClasses="max-w-md">
						{#snippet trigger()}
							{#if setting.select_items?.some((e) => e.label == $values[setting.key] || e.value == $values[setting.key])}
								<Button
									variant="default"
									btnClasses="px-1.5 py-1.5 text-2xs bg-surface-secondary border-0"
									nonCaptureEvent={true}>Select Custom</Button
								>
							{:else}
								<Button
									variant="default"
									btnClasses="px-1.5 py-1.5 text-2xs border-0 shadow-md"
									nonCaptureEvent={true}>Custom | {$values[setting.key]}</Button
								>
							{/if}
						{/snippet}
						{#snippet content()}
							{#if isPyFetching}
								<div class="p-4">
									<LoadingIcon />
								</div>
							{:else}
								<ToggleButtonGroup
									bind:selected={$values[setting.key]}
									class="mr-10 h-full"
									tabListClass="flex-wrap p-2"
								>
									{#snippet children({ item: toggleButtonn })}
										{#each pythonAvailableVersions as item}
											<ToggleButton value={item} label={item} tooltip={item} item={toggleButtonn} />
										{/each}
									{/snippet}
								</ToggleButtonGroup>
							{/if}
						{/snippet}
					</PopoverMelt>
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{:else}
		{#snippet settingContent()}
			<div class="flex flex-col gap-2 mb-1">
				<div class="flex items-center justify-between">
					<div class="text-emphasis font-semibold text-xs flex flex-col gap-1 w-full">
						<div class="flex items-center justify-between gap-2 w-full">
							{#if setting.fieldType != 'smtp_connect'}
								<div class="flex gap-1">
									<span class="text-emphasis font-semibold text-xs pb-1">{setting.label}</span>
									{#if setting.ee_only != undefined && !$enterpriseLicense}
										{#if setting.ee_only != ''}
											<EEOnly>{setting.ee_only}</EEOnly>
										{:else}
											<EEOnly />
										{/if}
									{/if}
								</div>
							{/if}
							{#if setting.actionButton}
								<Button
									disabled={setting.ee_only != undefined && !$enterpriseLicense}
									variant={setting.actionButton.variant ?? 'default'}
									unifiedSize="sm"
									onclick={async () => await setting.actionButton?.onclick($values)}
								>
									{setting.actionButton.label}
								</Button>
							{/if}
						</div>
						{#if setting.description}
							<span class="text-secondary font-normal text-xs">
								{@html setting.description}
							</span>
						{/if}
					</div>
				</div>
			</div>
			{#if setting.tooltip}
				<Tooltip>{setting.tooltip}</Tooltip>
			{/if}
			{#if $values}
				{@const hasError = setting.isValid && !setting.isValid($values[setting.key])}
				<div class="h-1"></div>
				{#if loading}
					<Skeleton layout={[[2.5]]} />
				{:else if setting.fieldType == 'text'}
					<input
						id={setting.key}
						disabled={setting.ee_only != undefined && !$enterpriseLicense}
						type="text"
						placeholder={setting.placeholder}
						class={hasError
							? 'border !border-red-700 !border-opacity-30 !focus:border-red-700 !focus:border-opacity-30'
							: ''}
						bind:value={$values[setting.key]}
					/>
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
							onKeyDown={() => {
								licenseKeyChanged = true
							}}
							onBlur={() => {
								if ($values[setting.key] && typeof $values[setting.key] === 'string') {
									$values[setting.key] = $values[setting.key].trim()
								}
							}}
							bind:password={$values[setting.key]}
						/>
						<Button
							variant="accent"
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
												<span class="text-red-300">
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
						{#if licenseKeyChanged && !$enterpriseLicense}
							{#if version.startsWith('CE')}
								<div class="text-red-400"
									>License key is set but image used is the Community Edition {version}. Switch
									image to EE.</div
								>
							{/if}
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
				{:else if setting.fieldType == 'indexer_rates'}
					<div class="flex flex-col gap-16 mt-4">
						{#if $values[setting.key]}
							<Section label="Memory" class="space-y-6">
								<div class="flex flex-col gap-1">
									<label
										for="writer_memory_budget"
										class="block text-xs font-semibold text-emphasis"
									>
										Index writer memory budget (MB)
										<Tooltip>
											The allocated memory arena for the indexer. A bigger value means less writing
											to disk and potentially higher indexing throughput
										</Tooltip>
									</label>
									<TextInput
										inputProps={{
											type: 'number',
											placeholder: '300',
											id: 'writer_memory_budget',
											disabled: !$enterpriseLicense,
											oninput: (e) => {
												if (e.target instanceof HTMLInputElement) {
													if (e.target.valueAsNumber) {
														$values[setting.key].writer_memory_budget =
															e.target.valueAsNumber * (1024 * 1024)
													}
												}
											}
										}}
										value={$values[setting.key].writer_memory_budget / (1024 * 1024)}
									/>
								</div>
								<Label label="Clear index">
									<span class="text-xs text-secondary"
										>This buttons will clear the whole index, and the service will start reindexing
										from scratch. Full text search might be down during this time.</span
									>
									<div class="flex flex-row gap-2">
										<Button
											variant="default"
											unifiedSize="sm"
											on:click={() => {
												clearJobsIndexModalOpen = true
											}}
										>
											Clear jobs index
										</Button>
										<Button
											variant="default"
											unifiedSize="sm"
											on:click={() => {
												clearServiceLogsIndexModalOpen = true
											}}
										>
											Clear service logs index
										</Button>
									</div>
								</Label>
								<ConfirmationModal
									title="Clear jobs index"
									confirmationText="Clear"
									open={clearJobsIndexModalOpen}
									type="danger"
									on:canceled={() => {
										clearJobsIndexModalOpen = false
									}}
									on:confirmed={async () => {
										const r = await IndexSearchService.clearIndex({
											idxName: 'JobIndex'
										})
										sendUserToast(r)
										clearJobsIndexModalOpen = false
									}}
								>
									Are you sure you want to clear the jobs index? The service will start reindexing
									from scratch. Full text search might be down during this time.
								</ConfirmationModal>
								<ConfirmationModal
									title="Clear service logs index"
									confirmationText="Clear"
									open={clearServiceLogsIndexModalOpen}
									type="danger"
									on:canceled={() => {
										clearServiceLogsIndexModalOpen = false
									}}
									on:confirmed={async () => {
										const r = await IndexSearchService.clearIndex({
											idxName: 'ServiceLogIndex'
										})
										sendUserToast(r)
										clearServiceLogsIndexModalOpen = false
									}}
								>
									Are you sure you want to clear the service logs index? The service will start
									reindexing from scratch. Full text search might be down during this time.
								</ConfirmationModal>
							</Section>
							<hr class="border-t -my-6" />
							<Section label="Completed Job Index" class="space-y-6">
								<div class="flex flex-col gap-1">
									<label
										for="commit_job_max_batch_size"
										class="block text-xs font-semibold text-emphasis"
									>
										Commit max batch size <Tooltip>
											The max amount of documents (here jobs) per commit. To optimize indexing
											throughput, it is best to keep this as high as possible. However, especially
											when reindexing the whole instance, it can be useful to have a limit on how
											many jobs can be written without being committed. A commit will make the jobs
											available for search, constitute a "checkpoint" state in the indexing and will
											be logged.
										</Tooltip>
									</label>
									<TextInput
										inputProps={{
											type: 'number',
											placeholder: '100000',
											id: 'commit_job_max_batch_size',
											disabled: !$enterpriseLicense
										}}
										bind:value={$values[setting.key].commit_job_max_batch_size}
									/>
								</div>
								<div class="flex flex-col gap-1">
									<label
										for="refresh_index_period"
										class="block text-xs font-semibold text-emphasis"
									>
										Refresh index period (s) <Tooltip>
											The index will query new jobs periodically and write them on the index. This
											setting sets that period.
										</Tooltip></label
									>
									<TextInput
										inputProps={{
											type: 'number',
											placeholder: '300',
											id: 'refresh_index_period',
											disabled: !$enterpriseLicense
										}}
										bind:value={$values[setting.key].refresh_index_period}
									/>
								</div>
								<div class="flex flex-col gap-1">
									<label
										for="max_indexed_job_log_size"
										class="block text-xs font-semibold text-emphasis"
									>
										Max indexed job log size (KB) <Tooltip>
											Job logs are included when indexing, but to avoid the index size growing
											artificially, the logs will be truncated after a size has been reached.
										</Tooltip>
									</label>
									<TextInput
										inputProps={{
											type: 'number',
											placeholder: '1024',
											id: 'max_indexed_job_log_size',
											disabled: !$enterpriseLicense,
											oninput: (e) => {
												if (e.target instanceof HTMLInputElement) {
													if (e.target.valueAsNumber) {
														$values[setting.key].max_indexed_job_log_size =
															e.target.valueAsNumber * 1024
													}
												}
											}
										}}
										value={$values[setting.key].max_indexed_job_log_size / 1024}
									/>
								</div>
							</Section>
							<hr class="border-t -my-6" />
							<Section label="Service logs index" class="space-y-6">
								<div class="flex flex-col gap-1">
									<label
										for="commit_log_max_batch_size"
										class="block text-xs font-semibold text-emphasis"
										>Commit max batch size <Tooltip>
											The max amount of documents per commit. In this case 1 document is one log
											file representing all logs during 1 minute for a specific host. To optimize
											indexing throughput, it is best to keep this as high as possible. However,
											especially when reindexing the whole instance, it can be useful to have a
											limit on how many logs can be written without being committed. A commit will
											make the logs available for search, appear as a log line, and be a
											"checkpoint" of the indexing progress.
										</Tooltip>
									</label>
									<input
										disabled={!$enterpriseLicense}
										type="number"
										id="commit_log_max_batch_size"
										placeholder="10000"
										bind:value={$values[setting.key].commit_log_max_batch_size}
									/>
								</div>

								<div class="flex flex-col gap-1">
									<label
										for="refresh_log_index_period"
										class="block text-xs font-semibold text-emphasis"
									>
										Refresh index period (s) <Tooltip>
											The index will query new service logs peridically and write them on the index.
											This setting sets that period.
										</Tooltip>
									</label>
									<TextInput
										inputProps={{
											type: 'number',
											placeholder: '300',
											id: 'refresh_log_index_period',
											disabled: !$enterpriseLicense
										}}
										bind:value={$values[setting.key].refresh_log_index_period}
									/>
								</div>
							</Section>
						{/if}
					</div>
				{:else if setting.fieldType == 'otel'}
					<div class="flex flex-col gap-4 border rounded p-4">
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
				{:else if setting.fieldType == 'object_store_config'}
					<ObjectStoreConfigSettings bind:bucket_config={$values[setting.key]} />
					<div class="mb-6"></div>
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
						<div class="mb-6"></div>
					{/if}
				{:else if setting.fieldType == 'number'}
					<TextInput
						inputProps={{
							type: 'number',
							placeholder: setting.placeholder,
							id: setting.key
						}}
						bind:value={$values[setting.key]}
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
						/>
					</div>
				{:else if setting.fieldType == 'select'}
					TODO
				{:else if setting.fieldType == 'smtp_connect'}
					<SmtpSettings {values} disabled={loading} />
				{/if}
				{#if hasError}
					<span class="text-red-500 dark:text-red-400 text-sm">
						{setting.error ?? ''}
					</span>
				{/if}
			{:else}
				<input disabled placeholder="Loading..." />
			{/if}
		{/snippet}

		<div class="block">
			{@render settingContent()}
		</div>
	{/if}
{/if}
