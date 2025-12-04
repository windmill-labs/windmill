<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense, isCriticalAlertsUIOpen } from '$lib/stores'
	import { AlertCircle, BadgeCheck, BadgeX, Info, Plus, Slack, X } from 'lucide-svelte'
	import type { Setting } from './instanceSettings'
	import Tooltip from './Tooltip.svelte'
	import ObjectStoreConfigSettings from './ObjectStoreConfigSettings.svelte'
	import { sendUserToast } from '$lib/toast'
	import ConfirmButton from './ConfirmButton.svelte'
	import {
		ConfigService,
		IndexSearchService,
		SettingService,
		type ListAvailablePythonVersionsResponse
	} from '$lib/gen'
	import { Button, SecondsInput, Skeleton } from './common'
	import Password from './Password.svelte'
	import { classNames } from '$lib/utils'
	import Popover from './Popover.svelte'
	import PopoverMelt from './meltComponents/Popover.svelte'
	import Toggle from './Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { base } from '$lib/base'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import LoadingIcon from './apps/svelte-select/lib/LoadingIcon.svelte'
	import TeamSelector from './TeamSelector.svelte'
	import ChannelSelector from './ChannelSelector.svelte'
	import EEOnly from './EEOnly.svelte'

	interface Props {
		setting: Setting
		version: string
		values: Writable<Record<string, any>>
		loading?: boolean
	}

	let { setting, version, values, loading = true }: Props = $props()
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

	let to: string = $state('')

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

	function handleTeamChange(
		teamItem: { team_id: string; team_name: string } | undefined,
		i: number
	) {
		const currentTeamChannel = $values['critical_error_channels'][i]?.teams_channel
		const teamIdChanged = currentTeamChannel?.team_id !== teamItem?.team_id

		$values['critical_error_channels'][i] = {
			teams_channel: teamItem
				? {
						team_id: teamItem.team_id,
						team_name: teamItem.team_name,
						// Preserve existing channel if team didn't actually change
						channel_id: teamIdChanged ? undefined : currentTeamChannel?.channel_id,
						channel_name: teamIdChanged ? undefined : currentTeamChannel?.channel_name
					}
				: undefined
		}
	}

	function handleChannelChange(
		channel: { channel_id?: string; channel_name?: string } | undefined,
		i: number
	) {
		const team = $values['critical_error_channels'][i]?.teams_channel
		if (team) {
			$values['critical_error_channels'][i] = {
				teams_channel: {
					team_id: team?.team_id,
					team_name: team?.team_name,
					channel_id: channel?.channel_id,
					channel_name: channel?.channel_name
				}
			}
		}
	}
</script>

<!-- {JSON.stringify($values, null, 2)} -->
{#if (!setting.cloudonly || isCloudHosted()) && showSetting(setting.key, $values) && !(setting.hiddenIfNull && $values[setting.key] == null) && !(setting.hiddenIfEmpty && !$values[setting.key])}
	{#if setting.ee_only != undefined && !$enterpriseLicense}
		<EEOnly>
			{#if setting.ee_only != ''}{setting.ee_only}{/if}
		</EEOnly>
	{/if}
	{#if setting.fieldType == 'select'}
		<div>
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<label class="block pb-1">
				<span class="text-emphasis font-semibold text-xs pb-1">{setting.label}</span>
				{#if setting.description}
					<span class="text-secondary text-xs font-normal">
						{@html setting.description}
					</span>
				{/if}
			</label>
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
			<label class="mb-1 flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">{setting.label}</span>
				{#if setting.description}
					<span class="text-secondary text-xs font-normal">
						{@html setting.description}
					</span>
				{/if}
			</label>

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
			<div class="text-emphasis font-semibold text-xs flex flex-col gap-1 mb-1"
				>{setting.label}
				{#if setting.description}
					<span class="text-secondary font-normal text-xs">
						{@html setting.description}
					</span>
				{/if}
			</div>
			{#if setting.tooltip}
				<Tooltip>{setting.tooltip}</Tooltip>
			{/if}
			{#if $values}
				{@const hasError = setting.isValid && !setting.isValid($values[setting.key])}
				{#if loading}
					<Skeleton layout={[[2.5]]} />
				{:else if setting.fieldType == 'text'}
					<input
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
					<input type="email" placeholder={setting.placeholder} bind:value={$values[setting.key]} />
				{:else if setting.key == 'critical_alert_mute_ui'}
					<div class="flex flex-col gap-y-2 my-2 py-2">
						<Toggle
							disabled={!$enterpriseLicense}
							bind:checked={$values[setting.key]}
							options={{ right: setting.description }}
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
					<div class="w-full flex gap-x-16 flex-wrap">
						<div class="w-full max-w-lg">
							{#if $enterpriseLicense && Array.isArray($values[setting.key])}
								{#each $values[setting.key] ?? [] as v, i}
									<div class="flex w-full max-w-lg mt-1 gap-2 items-center">
										<select
											class="max-w-24"
											onchange={(e) => {
												if (e.target?.['value']) {
													$values[setting.key][i] = {
														[e.target['value']]: ''
													}
												}
											}}
											value={(() => {
												if (!v) return 'email'
												return (
													['slack_channel', 'teams_channel'].find((type) => type in v) || 'email'
												)
											})()}
										>
											<option value="email">Email</option>
											<option value="slack_channel">Slack</option>
											<option value="teams_channel">Teams</option>
										</select>
										{#if v && 'slack_channel' in v}
											<input
												type="text"
												placeholder="Slack channel"
												oninput={(e) => {
													if (e.target?.['value']) {
														$values[setting.key][i] = {
															slack_channel: e.target['value']
														}
													}
												}}
												value={v?.slack_channel ?? ''}
											/>
										{:else if v && 'teams_channel' in v}
											{@const currentTeam = $values['critical_error_channels'][i]?.teams_channel
												? {
														team_id: $values['critical_error_channels'][i]?.teams_channel?.team_id,
														team_name:
															$values['critical_error_channels'][i]?.teams_channel?.team_name
													}
												: undefined}
											{@const currentChannel = $values['critical_error_channels'][i]?.teams_channel
												?.channel_id
												? {
														channel_id:
															$values['critical_error_channels'][i]?.teams_channel?.channel_id,
														channel_name:
															$values['critical_error_channels'][i]?.teams_channel?.channel_name
													}
												: undefined}
											<div class="flex flex-row gap-2 w-full">
												<TeamSelector
													containerClass="w-44"
													minWidth="140px"
													showRefreshButton={false}
													selectedTeam={currentTeam}
													onSelectedTeamChange={(team) => handleTeamChange(team, i)}
												/>

												{#if $values['critical_error_channels'][i]?.teams_channel?.team_id}
													<ChannelSelector
														containerClass=""
														placeholder="Search channels"
														teamId={$values['critical_error_channels'][i]?.teams_channel?.team_id}
														selectedChannel={currentChannel}
														onSelectedChannelChange={(channel) => handleChannelChange(channel, i)}
														onError={(e) =>
															sendUserToast('Failed to load channels: ' + e.message, true)}
													/>
												{/if}
											</div>
										{:else}
											<input
												type="email"
												placeholder="Email address"
												oninput={(e) => {
													if (e.target?.['value']) {
														$values[setting.key][i] = {
															email: e.target['value']
														}
													}
												}}
												value={v?.email ?? ''}
											/>
										{/if}
										<button
											transition:fade|local={{ duration: 100 }}
											class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
											aria-label="Clear"
											onclick={() => {
												$values[setting.key] = $values[setting.key].filter(
													(_, index) => index !== i
												)
											}}
										>
											<X size={14} />
										</button>
									</div>
								{/each}
							{/if}
						</div>
					</div>
					<div class="flex mt-2 gap-20 items-center">
						<Button
							variant="default"
							size="xs"
							btnClasses="mt-1"
							on:click={() => {
								if ($values[setting.key] == undefined || !Array.isArray($values[setting.key])) {
									$values[setting.key] = []
								}
								// Start with a typed default to avoid invalid primitives in the array
								$values[setting.key] = $values[setting.key].concat({ email: '' })
							}}
							id="arg-input-add-item"
							startIcon={{ icon: Plus }}
							disabled={!$enterpriseLicense}
						>
							Add channel
						</Button>
						<div class="flex mt-1">
							<Button
								disabled={!$enterpriseLicense}
								variant="default"
								size="xs"
								on:click={async () => {
									try {
										await SettingService.testCriticalChannels({
											requestBody: $values[setting.key]
										})
										sendUserToast('Test message sent successfully to critical channels', false)
									} catch (error) {
										sendUserToast('Failed to send test message: ' + error.message, true)
									}
								}}
							>
								Test channels
							</Button>
						</div>
					</div>
				{:else if setting.fieldType == 'slack_connect'}
					<div class="flex flex-col items-start self-start">
						{#if $values[setting.key] && 'team_name' in $values[setting.key]}
							<div class="text-sm">
								Connected to <code>{$values[setting.key]['team_name']}</code>
							</div>
							<Button
								size="sm"
								endIcon={{ icon: Slack }}
								btnClasses="mt-2"
								variant="default"
								on:click={async () => {
									$values[setting.key] = undefined
								}}
							>
								Disconnect Slack
							</Button>
						{:else}
							<Button
								size="xs"
								variant="accent"
								href="{base}/api/oauth/connect_slack?instance=true"
								startIcon={{ icon: Slack }}
								disabled={!$enterpriseLicense}
							>
								Connect to Slack
							</Button>
						{/if}
					</div>
				{:else if setting.fieldType == 'indexer_rates'}
					<div class="flex flex-col gap-4 mt-4">
						{#if $values[setting.key]}
							<div>
								<label for="writer_memory_budget" class="block text-xs font-semibold text-emphasis">
									Index writer memory budget (MB)
									<Tooltip>
										The allocated memory arena for the indexer. A bigger value means less writing to
										disk and potentially higher indexing throughput
									</Tooltip>
								</label>
								<input
									disabled={!$enterpriseLicense}
									type="number"
									id="writer_memory_budget"
									placeholder="300"
									oninput={(e) => {
										if (e.target instanceof HTMLInputElement) {
											if (e.target.valueAsNumber) {
												$values[setting.key].writer_memory_budget =
													e.target.valueAsNumber * (1024 * 1024)
											}
										}
									}}
									value={$values[setting.key].writer_memory_budget / (1024 * 1024)}
								/>
							</div>
							<h3 class="text-sm font-semibold text-emphasis mt-8">Completed Job Index</h3>
							<div>
								<label
									for="commit_job_max_batch_size"
									class="block text-xs font-semibold text-emphasis"
								>
									Commit max batch size <Tooltip>
										The max amount of documents (here jobs) per commit. To optimize indexing
										throughput, it is best to keep this as high as possible. However, especially
										when reindexing the whole instance, it can be useful to have a limit on how many
										jobs can be written without being committed. A commit will make the jobs
										available for search, constitute a "checkpoint" state in the indexing and will
										be logged.
									</Tooltip>
								</label>
								<input
									disabled={!$enterpriseLicense}
									type="number"
									id="commit_job_max_batch_size"
									placeholder="100000"
									bind:value={$values[setting.key].commit_job_max_batch_size}
								/>
							</div>
							<div>
								<label for="refresh_index_period" class="block text-xs font-semibold text-emphasis">
									Refresh index period (s) <Tooltip>
										The index will query new jobs periodically and write them on the index. This
										setting sets that period.
									</Tooltip></label
								>
								<input
									disabled={!$enterpriseLicense}
									type="number"
									id="refresh_index_period"
									placeholder="300"
									bind:value={$values[setting.key].refresh_index_period}
								/>
							</div>
							<div>
								<label
									for="max_indexed_job_log_size"
									class="block text-xs font-semibold text-emphasis"
								>
									Max indexed job log size (KB) <Tooltip>
										Job logs are included when indexing, but to avoid the index size growing
										artificially, the logs will be truncated after a size has been reached.
									</Tooltip>
								</label>
								<input
									disabled={!$enterpriseLicense}
									type="number"
									id="max_indexed_job_log_size"
									placeholder="1024"
									oninput={(e) => {
										if (e.target instanceof HTMLInputElement) {
											if (e.target.valueAsNumber) {
												$values[setting.key].max_indexed_job_log_size =
													e.target.valueAsNumber * 1024
											}
										}
									}}
									value={$values[setting.key].max_indexed_job_log_size / 1024}
								/>
							</div>
							<h3 class="text-sm font-semibold text-emphasis mt-8">Service logs index</h3>
							<div>
								<label
									for="commit_log_max_batch_size"
									class="block text-xs font-semibold text-emphasis"
									>Commit max batch size <Tooltip>
										The max amount of documents per commit. In this case 1 document is one log file
										representing all logs during 1 minute for a specific host. To optimize indexing
										throughput, it is best to keep this as high as possible. However, especially
										when reindexing the whole instance, it can be useful to have a limit on how many
										logs can be written without being committed. A commit will make the logs
										available for search, appear as a log line, and be a "checkpoint" of the
										indexing progress.
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
							<div>
								<label
									for="refresh_log_index_period"
									class="block text-xs font-semibold text-emphasis"
								>
									Refresh index period (s) <Tooltip>
										The index will query new service logs peridically and write them on the index.
										This setting sets that period.
									</Tooltip></label
								>
								<input
									disabled={!$enterpriseLicense}
									type="number"
									id="refresh_log_index_period"
									placeholder="300"
									bind:value={$values[setting.key].refresh_log_index_period}
								/>
							</div>
							<h3 class="text-sm font-semibold text-emphasis mt-8">Reset Index</h3>
							<p class="text-xs text-primary font-normal">
								This buttons will clear the whole index, and the service will start reindexing from
								scratch. Full text search might be down during this time.
							</p>
							<div>
								<ConfirmButton
									on:click={async () => {
										let r = await IndexSearchService.clearIndex({
											idxName: 'JobIndex'
										})
										console.log('asasd')
										sendUserToast(r)
									}}>Clear <b>Jobs</b> Index</ConfirmButton
								>
								<ConfirmButton
									on:click={async () => {
										let r = await IndexSearchService.clearIndex({
											idxName: 'ServiceLogIndex'
										})
										console.log('asasd')
										sendUserToast(r)
									}}>Clear <b>Service logs</b> index</ConfirmButton
								>
							</div>
						{/if}
					</div>
				{:else if setting.fieldType == 'smtp_connect'}
					<div class="flex flex-col gap-4 border rounded p-4">
						{#if $values[setting.key]}
							<div class="flex gap-4">
								<input type="email" bind:value={to} placeholder="contact@windmill.dev" />
								<Button
									wrapperClasses="shrink-0"
									disabled={to == ''}
									on:click={async () => {
										let smtp = $values[setting.key]
										await SettingService.testSmtp({
											requestBody: {
												to,
												smtp: {
													host: smtp['smtp_host'],
													username: smtp['smtp_username'],
													password: smtp['smtp_password'],
													port: smtp['smtp_port'],
													from: smtp['smtp_from'],
													tls_implicit: smtp['smtp_tls_implicit'],
													disable_tls: smtp['smtp_disable_tls']
												}
											}
										})
										sendUserToast('Test email sent')
									}}>Test SMTP settings</Button
								></div
							>
							<div>
								<label for="smtp_host" class="block text-xs font-semibold text-emphasis">Host</label
								>
								<input
									type="text"
									id="smtp_host"
									placeholder="smtp.gmail.com"
									bind:value={$values[setting.key].smtp_host}
								/>
							</div>
							<div>
								<label for="smtp_port" class="block text-xs font-semibold text-emphasis">Port</label
								>
								<input
									type="number"
									id="smtp_port"
									placeholder="587"
									bind:value={$values[setting.key].smtp_port}
								/>
							</div>
							<div>
								<label for="smtp_username" class="block text-xs font-semibold text-emphasis"
									>Username</label
								>
								<input
									type="text"
									id="smtp_username"
									placeholder="ruben@windmill.dev"
									bind:value={$values[setting.key].smtp_username}
								/>
							</div>
							<div>
								<label for="smtp_password" class="block text-xs font-semibold text-emphasis"
									>Password</label
								>
								<Password bind:password={$values[setting.key].smtp_password} />
							</div>
							<div>
								<label for="smtp_from" class="block text-xs font-semibold text-emphasis"
									>From Address</label
								>
								<input
									type="email"
									id="smtp_from"
									placeholder="noreply@windmill.dev"
									bind:value={$values[setting.key].smtp_from}
								/>
							</div>
							<div>
								<Toggle
									disabled={$values[setting.key].smtp_disable_tls == true || !$enterpriseLicense}
									id="smtp_tls_implicit"
									bind:checked={$values[setting.key].smtp_tls_implicit}
									options={{ right: 'Implicit TLS' }}
								/>
							</div>
							<div>
								<Toggle
									id="smtp_disable_tls"
									disabled={!$enterpriseLicense}
									bind:checked={$values[setting.key].smtp_disable_tls}
									on:change={() => {
										if ($values[setting.key].smtp_disable_tls) {
											$values[setting.key].smtp_tls_implicit = false
										}
									}}
									options={{ right: 'Disable TLS' }}
								/>
							</div>
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

							<div>
								<label
									for="OTEL_EXPORTER_OTLP_ENDPOINT"
									class="block text-xs font-semibold text-emphasis">Endpoint</label
								>
								<input
									disabled={!$enterpriseLicense}
									type="text"
									id="OTEL_EXPORTER_OTLP_ENDPOINT"
									placeholder="http://otel-collector.example.com:4317"
									bind:value={$values[setting.key].otel_exporter_otlp_endpoint}
								/>
							</div>
							<div>
								<label
									for="OTEL_EXPORTER_OTLP_HEADERS"
									class="block text-xs font-semibold text-emphasis">Headers</label
								>
								<input
									disabled={!$enterpriseLicense}
									type="text"
									id="OTEL_EXPORTER_OTLP_HEADERS"
									placeholder="Authorization=Bearer my-secret-token,Env=production"
									bind:value={$values[setting.key].otel_exporter_otlp_headers}
								/>
							</div>
							<div>
								<label
									for="OTEL_EXPORTER_OTLP_PROTOCOL"
									class="block text-xs font-semibold text-emphasis">Protocol</label
								>
								gRPC
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
					<input
						type="number"
						placeholder={setting.placeholder}
						bind:value={$values[setting.key]}
					/>
				{:else if setting.fieldType == 'password'}
					<Password small placeholder={setting.placeholder} bind:password={$values[setting.key]} />
				{:else if setting.fieldType == 'boolean'}
					<div class="mt-0.5">
						<Toggle
							disabled={setting.ee_only != undefined && !$enterpriseLicense}
							bind:checked={$values[setting.key]}
						/>
					</div>
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
		{#if ['codearea', 'object_store_config'].includes(setting.fieldType)}
			<div class="block pb-2">
				{@render settingContent()}
			</div>
		{:else}
			<label class="block pb-2">
				{@render settingContent()}
			</label>
		{/if}
	{/if}
{/if}
