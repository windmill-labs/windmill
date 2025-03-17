<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense, isCriticalAlertsUIOpen } from '$lib/stores'
	import {
		AlertCircle,
		AlertTriangle,
		BadgeCheck,
		BadgeX,
		Info,
		Plus,
		RefreshCcw,
		Slack,
		X
	} from 'lucide-svelte'
	import type { Setting } from './instanceSettings'
	import Tooltip from './Tooltip.svelte'
	import ObjectStoreConfigSettings from './ObjectStoreConfigSettings.svelte'
	import { sendUserToast } from '$lib/toast'
	import ConfirmButton from './ConfirmButton.svelte'
	import {
		IndexSearchService,
		SettingService,
		TeamsService,
		WorkspaceService,
		GitSyncService
	} from '$lib/gen'
	import type {
		GithubInstallations,
		WorkspaceGithubInstallation,
		Workspace
	} from '$lib/gen/types.gen'
	import { Button, SecondsInput, Skeleton } from './common'
	import Password from './Password.svelte'
	import { classNames } from '$lib/utils'
	import Popover from './meltComponents/Popover.svelte'
	import Toggle from './Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { base } from '$lib/base'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from './SimpleEditor.svelte'

	export let setting: Setting
	export let version: string
	export let values: Writable<Record<string, any>>
	export let loading = true
	const dispatch = createEventDispatcher()

	if (setting.fieldType == 'select' && $values[setting.key] == undefined) {
		$values[setting.key] = 'default'
	}

	let latestKeyRenewalAttempt: {
		result: string
		attempted_at: string
	} | null

	let isFetching = false

	function showSetting(setting: string, values: Record<string, any>) {
		if (setting == 'dev_instance') {
			if (values['license_key'] == undefined) {
				return false
			}
		}
		return true
	}

	let licenseKeyChanged = false
	let renewing = false
	let opening = false

	let to: string = ''

	async function reloadKeyrenewalAttemptInfo() {
		latestKeyRenewalAttempt = await SettingService.getLatestKeyRenewalAttempt()
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
			reloadKeyrenewalAttemptInfo()
		} catch (err) {
			latestKeyRenewalAttempt = await SettingService.getLatestKeyRenewalAttempt()
			throw err
		} finally {
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

	async function fetchTeams() {
		if (isFetching) return
		isFetching = true
		try {
			$values['teams'] = await TeamsService.syncTeams()
		} catch (error) {
			console.error('Error fetching teams:', error)
		} finally {
			isFetching = false
		}
	}

	function handleTeamChange(event: Event, i: number) {
		const teamId = (event.target as HTMLSelectElement).value
		const team = $values['teams'].find((team) => team.team_id === teamId) || null
		$values['critical_error_channels'][i] = {
			teams_channel: {
				team_id: team?.team_id,
				team_name: team?.team_name,
				channel_id: team?.channels[0]?.channel_id,
				channel_name: team?.channels[0]?.channel_name
			}
		}
	}

	function handleChannelChange(event: Event, setting: Setting, i: number) {
		const channelId = (event.target as HTMLSelectElement).value
		const team = $values['teams'].find(
			(team) => team.team_id === $values['critical_error_channels'][i]?.teams_channel?.team_id
		)
		const channel = team?.channels.find((channel) => channel.channel_id === channelId) || null
		if (channelId) {
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

	let selectedWorkspace = ''
	let selectedOrganization = ''
	let workspaces: Workspace[] = []
	let gitOrganizations: GithubInstallations = []
	let gitInstallationsOpen = false

	$: (($enterpriseLicense && $values['git_installations'] && setting.key == 'git_installations') ||
		gitInstallationsOpen) &&
		(async () => {
			await getWorkspaces()
			await getInstallations()
		})()

	async function getWorkspaces() {
		workspaces = await WorkspaceService.listWorkspacesAsSuperAdmin()
	}

	async function getInstallations() {
		if (!$enterpriseLicense) return
		gitOrganizations = await GitSyncService.getGlobalConnectedRepositories()
	}

	async function addInstallation(close: (_: any) => void) {
		if (!selectedWorkspace || !selectedOrganization) return

		if (selectedOrganization == 'new') {
			const url = await GitSyncService.getGithubAppInstallationUrl({
				workspace: selectedWorkspace
			})
			if (url) {
				window.open(url.installation_url, '_blank')
			} else {
				sendUserToast('Failed to get GitHub app installation URL', true)
			}
			return
		}

		const installations = $values['git_installations'] as Record<
			string,
			WorkspaceGithubInstallation[]
		>
		const workspaceInstalls =
			(installations[selectedWorkspace] as WorkspaceGithubInstallation[]) || []

		// Find the installation ID for the selected organization
		const orgInstallation = gitOrganizations
			.flat()
			.find((install) => install.account_id === selectedOrganization)

		if (!orgInstallation) return

		// Add new installation to the workspace
		const newInstallation = {
			account_id: selectedOrganization,
			installation_id: orgInstallation.installation_id
		}

		installations[selectedWorkspace] = [...workspaceInstalls, newInstallation]
		$values['git_installations'] = installations

		// Reset form and close popover
		selectedWorkspace = ''
		selectedOrganization = ''
		close(null)
	}

	$: typedInstallations = $values['git_installations'] as Record<
		string,
		WorkspaceGithubInstallation[]
	>
	$: typedInstallations && console.log('typedInstallations', typedInstallations)
</script>

<!-- {JSON.stringify($values, null, 2)} -->
{#if (!setting.cloudonly || isCloudHosted()) && showSetting(setting.key, $values) && !(setting.hiddenIfNull && $values[setting.key] == null)}
	{#if setting.ee_only != undefined && !$enterpriseLicense}
		<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap">
			<AlertTriangle size={16} />
			EE only {#if setting.ee_only != ''}<Tooltip>{setting.ee_only}</Tooltip>{/if}
		</div>
	{/if}
	{#if setting.fieldType == 'select'}
		<div>
			<!-- svelte-ignore a11y-label-has-associated-control -->
			<label class="block pb-2">
				<span class="text-primary font-semibold text-sm">{setting.label}</span>
				{#if setting.description}
					<span class="text-secondary text-xs">
						{@html setting.description}
					</span>
				{/if}
			</label>
			<ToggleButtonGroup bind:selected={$values[setting.key]} let:item={toggleButton}>
				{#each setting.select_items ?? [] as item}
					<ToggleButton
						value={item.value ?? item.label}
						label={item.label}
						tooltip={item.tooltip}
						item={toggleButton}
					/>
				{/each}
			</ToggleButtonGroup>
		</div>
	{:else}
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">{setting.label}</span>
			{#if setting.description}
				<span class="text-secondary text-xs">
					{@html setting.description}
				</span>
			{/if}
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
					/>
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
							on:keydown={() => {
								licenseKeyChanged = true
							}}
							bind:password={$values[setting.key]}
						/>
						<Button
							variant={$values[setting.key] ? 'contained' : 'border'}
							size="xs"
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
									<Info size={12} class="text-tertiary" />
									<span class="text-tertiary text-xs"
										>License key expires on {expiration ?? ''}</span
									>
								</div>
							{:else if expiration}
								<div class="flex flex-row gap-1 items-center">
									<AlertCircle size={12} class="text-red-600" />
									<span class="text-red-600 dark:text-red-400 text-xs"
										>License key expired on {expiration}</span
									>
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
											{latestKeyRenewalAttempt.result === 'success'
												? 'Latest key renewal succeeded'
												: isTrial
												? 'Latest key renewal ignored because in trial'
												: 'Latest key renewal failed'}
											on {attemptedAt}
										</span>
									</div>
									<div slot="text">
										{#if latestKeyRenewalAttempt.result === 'success'}
											<span class="text-green-300">
												Latest key renewal succeeded on {attemptedAt}
											</span>
										{:else if isTrial}
											<span class="text-yellow-300">
												License key cannot be renewed during trial ({attemptedAt})
											</span>
										{:else}
											<span class="text-red-300">
												Latest key renewal failed on {attemptedAt}: {latestKeyRenewalAttempt.result.replace(
													'error: ',
													''
												)}
											</span>
										{/if}
										<br />
										As long as invoices are paid and usage corresponds to the subscription, the key is
										renewed daily with a validity of 35 days (grace period).
									</div>
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
								<Button on:click={renewLicenseKey} loading={renewing} size="xs" color="dark"
									>Renew key
								</Button>
								<Button color="dark" size="xs" loading={opening} on:click={openCustomerPortal}>
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
								variant="border"
								color="light"
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
											on:change={(e) => {
												if (e.target?.['value']) {
													$values[setting.key][i] = {
														[e.target['value']]: ''
													}
												}
												if (e.target?.['value'] === 'teams_channel') {
													fetchTeams()
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
												on:input={(e) => {
													if (e.target?.['value']) {
														$values[setting.key][i] = {
															slack_channel: e.target['value']
														}
													}
												}}
												value={v?.slack_channel ?? ''}
											/>
										{:else if v && 'teams_channel' in v}
											<div class="flex flex-row gap-2 w-full">
												<select on:change={(e) => handleTeamChange(e, i)}>
													<option
														value=""
														disabled
														selected={!$values['critical_error_channels'][i]?.teams_channel
															?.team_id}>Select team</option
													>
													{#if $values['teams']}
														{#each $values['teams'] as team}
															<option
																value={team.team_id}
																selected={$values['critical_error_channels'][i]?.teams_channel
																	?.team_id === team.team_id}
															>
																{team.team_name}
															</option>
														{/each}
													{/if}
												</select>
												{#if $values['critical_error_channels'][i]?.teams_channel?.team_id}
													<select
														id="channel-select"
														on:change={(e) => handleChannelChange(e, setting, i)}
													>
														<option
															value=""
															disabled
															selected={!$values['critical_error_channels'][i]?.teams_channel
																?.channel_id}>Select channel</option
														>
														{#each $values['teams'].find((team) => team.team_id === $values['critical_error_channels'][i]?.teams_channel?.team_id)?.channels ?? [] as channel}
															<option
																value={channel.channel_id}
																selected={$values['critical_error_channels'][i]?.teams_channel
																	?.channel_id === channel.channel_id}
															>
																{channel.channel_name}
															</option>
														{/each}
													</select>
												{/if}
												<div>
													<button on:click={fetchTeams} class="flex items-center gap-1 mt-2">
														<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
													</button>
												</div>
											</div>
										{:else}
											<input
												type="email"
												placeholder="Email address"
												on:input={(e) => {
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
											on:click={() => {
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
							variant="border"
							color="light"
							size="md"
							btnClasses="mt-1"
							on:click={() => {
								if ($values[setting.key] == undefined || !Array.isArray($values[setting.key])) {
									$values[setting.key] = []
								}
								$values[setting.key] = $values[setting.key].concat('')
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
								variant="border"
								color="light"
								size="md"
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
								variant="border"
								on:click={async () => {
									$values[setting.key] = undefined
								}}
							>
								Disconnect Slack
							</Button>
						{:else}
							<Button
								size="xs"
								color="dark"
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
								<label for="writer_memory_budget" class="block text-sm font-medium">
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
									on:input={(e) => {
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
							<h3>Completed Job Index</h3>
							<div>
								<label for="commit_job_max_batch_size" class="block text-sm font-medium">
									Commit max batch size <Tooltip>
										The max amount of documents (here jobs) per commit. To optimize indexing
										throughput, it is best to keep this as high as possible. However, especially
										when reindexing the whole instance, it can be useful to have a limit on how many
										jobs can be written without being commited. A commit will make the jobs
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
								<label for="refresh_index_period" class="block text-sm font-medium">
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
								<label for="max_indexed_job_log_size" class="block text-sm font-medium">
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
									on:input={(e) => {
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
							<h3>Service Logs Index</h3>
							<div>
								<label for="commit_log_max_batch_size" class="block text-sm font-medium"
									>Commit max batch size <Tooltip>
										The max amount of documents per commit. In this case 1 document is one log file
										representing all logs during 1 minute for a specific host. To optimize indexing
										throughput, it is best to keep this as high as possible. However, especially
										when reindexing the whole instance, it can be useful to have a limit on how many
										logs can be written without being commited. A commit will make the logs
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
								<label for="refresh_log_index_period" class="block text-sm font-medium">
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
							<h3>Reset Index</h3>
							This buttons will clear the whole index, and the service will start reindexing from scratch.
							Full text search might be down during this time.
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
									}}>Clear <b>Service Logs</b> Index</ConfirmButton
								>
							</div>
						{/if}
					</div>
				{:else if setting.fieldType == 'smtp_connect'}
					<div class="flex flex-col gap-4 border rounded p-4">
						{#if $values[setting.key]}
							<div class="flex gap-4"
								><input type="email" bind:value={to} placeholder="contact@windmill.dev" />
								<Button
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
								<label for="smtp_host" class="block text-sm font-medium">Host</label>
								<input
									type="text"
									id="smtp_host"
									placeholder="smtp.gmail.com"
									bind:value={$values[setting.key].smtp_host}
								/>
							</div>
							<div>
								<label for="smtp_port" class="block text-sm font-medium">Port</label>
								<input
									type="number"
									id="smtp_port"
									placeholder="587"
									bind:value={$values[setting.key].smtp_port}
								/>
							</div>
							<div>
								<label for="smtp_username" class="block text-sm font-medium">Username</label>
								<input
									type="text"
									id="smtp_username"
									placeholder="ruben@windmill.dev"
									bind:value={$values[setting.key].smtp_username}
								/>
							</div>
							<div>
								<label for="smtp_password" class="block text-sm font-medium">Password</label>
								<Password bind:password={$values[setting.key].smtp_password} />
							</div>
							<div>
								<label for="smtp_from" class="block text-sm font-medium">From Address</label>
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
									label="Implicit TLS"
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
									label="Disable TLS"
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
									label="Tracing"
								/>
								<Toggle
									disabled={!$enterpriseLicense}
									id="logs_enabled"
									bind:checked={$values[setting.key].logs_enabled}
									options={{ right: 'Logs' }}
									label="logs"
								/>
								<Toggle
									disabled
									id="metrics_enabled"
									bind:checked={$values[setting.key].logs_enabled}
									options={{ right: 'Metrics (coming soon)' }}
									label="metrics"
								/>
							</div>

							<div>
								<label for="OTEL_EXPORTER_OTLP_ENDPOINT" class="block text-sm font-medium"
									>Endpoint</label
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
								<label for="OTEL_EXPORTER_OTLP_HEADERS" class="block text-sm font-medium"
									>Headers</label
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
								<label for="OTEL_EXPORTER_OTLP_PROTOCOL" class="block text-sm font-medium"
									>Protocol</label
								>
								gRPC
							</div>
							<!-- <div>
							<label for="OTEL_EXPORTER_OTLP_PROTOCOL" class="block text-sm font-medium"
								>Protocol<span class="text-2xs text-tertiary ml-4"
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
							<label for="OTEL_EXPORTER_OTLP_COMPRESSION" class="block text-sm font-medium"
								>Compression <span class="text-2xs text-tertiary ml-4">none, gzip</span></label
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
					<div class="mb-6" />
				{:else if setting.fieldType == 'number'}
					<input
						type="number"
						placeholder={setting.placeholder}
						bind:value={$values[setting.key]}
					/>
				{:else if setting.fieldType == 'password'}
					<input
						autocomplete="new-password"
						type="password"
						placeholder={setting.placeholder}
						bind:value={$values[setting.key]}
					/>
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
				{:else if setting.fieldType == 'git_installations'}
					<div class="flex my-2">
						<Popover
							floatingConfig={{
								placement: 'right'
							}}
							bind:isOpen={gitInstallationsOpen}
							disabled={!$enterpriseLicense}
						>
							<svelte:fragment slot="trigger">
								<Button
									spacingSize="sm"
									size="xs"
									btnClasses="h-8"
									color="light"
									disabled={!$enterpriseLicense}
									variant="border"
									nonCaptureEvent
								>
									Add GitHub Organization to Workspace
								</Button>
							</svelte:fragment>
							<svelte:fragment slot="content" let:close>
								<div class="block text-primary p-4">
									<div class="w-[550px] flex flex-col items-start gap-4">
										<div class="flex flex-row gap-2 w-full">
											<div class="flex flex-col gap-1 flex-1">
												<p class="text-sm font-semibold text-secondary">Workspace</p>
												<select
													bind:value={selectedWorkspace}
													class="w-full rounded-md border-gray-300 text-sm"
												>
													<option value="">Select workspace...</option>
													{#each workspaces as workspace}
														<option value={workspace.id}>{workspace.name}</option>
													{/each}
												</select>
											</div>
											<div class="flex flex-col gap-1 flex-1">
												<p class="text-sm font-semibold text-secondary">GitHub Organization</p>
												<select
													bind:value={selectedOrganization}
													class="w-full rounded-md border-gray-300 text-sm"
												>
													<option value="">Select organization...</option>
													<option value="new"> + New Installation</option>
													{#each gitOrganizations as org}
														<option value={org.account_id}>{org.account_id}</option>
													{/each}
												</select>
											</div>
											<div class="pt-[24px]">
												<Button
													size="xs"
													color="blue"
													buttonType="button"
													disabled={!selectedWorkspace || !selectedOrganization}
													on:click={() => addInstallation(close)}
												>
													Add
												</Button>
											</div>
										</div>
									</div>
								</div>
							</svelte:fragment>
						</Popover>
					</div>
					{#if !$values['git_installations'] || Object.keys($values['git_installations']).length === 0}
						<div role="presentation" on:click|stopPropagation|preventDefault>
							<div class="text-gray-500 p-4 rounded-md bg-gray-50">
								No GitHub App installations found.
							</div>
						</div>
					{:else}
						<div class="flex flex-col gap-4">
							<div class="overflow-x-auto rounded-lg border border-gray-200">
								<div role="presentation" on:click|stopPropagation|preventDefault>
									<table class="min-w-full divide-y divide-gray-200 table-fixed">
										<thead class="bg-gray-50">
											<tr>
												<th
													class="w-1/6 px-6 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
												>
													Workspace
												</th>
												<th
													class="w-1/6 px-6 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
												>
													Organization
												</th>
												<th
													class="w-3/6 px-6 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
												>
													Connected Repositories
													<Tooltip>
														<span>Repositories that are connected to this installation</span>
													</Tooltip>
												</th>
												<th class="w-1/12 px-6 py-2" />
											</tr>
										</thead>
										<tbody class="bg-white divide-y divide-gray-200">
											{#each Object.entries(typedInstallations) as [workspace, installations]}
												{@const workspace_obj = workspaces.find((w) => w.id === workspace)}
												{#each installations as installation}
													{@const repos =
														gitOrganizations.find(
															(i) => i.installation_id === installation.installation_id
														)?.repositories ?? []}
													<tr class="hover:bg-gray-50">
														<td class="px-6 py-2 whitespace-nowrap text-sm text-gray-900">
															{#if workspace_obj}
																<span
																	class="px-2 py-0.5 rounded"
																	style:background-color={workspace_obj.color + '20'}
																>
																	{workspace_obj.name}
																</span>
															{/if}
														</td>
														<td class="px-6 py-2 whitespace-nowrap text-sm text-gray-900">
															<a
																href={`https://github.com/${installation.account_id}`}
																target="_blank"
															>
																{installation.account_id}
															</a>
														</td>
														<td class="px-6 py-2 whitespace-nowrap text-sm text-gray-900">
															{#if repos.length > 0}
																<Popover>
																	<svelte:fragment slot="trigger">
																		<button class="text-blue-500 hover:underline">
																			Show {repos.length} repositories
																		</button>
																	</svelte:fragment>
																	<svelte:fragment slot="content">
																		<div class="p-2 max-w-md">
																			{#each repos as repo}
																				<div class="py-1">
																					<a
																						href={`https://github.com/${installation.account_id}/${repo.name}`}
																						target="_blank"
																						class="text-blue-500 hover:underline"
																					>
																						{repo.name}
																					</a>
																				</div>
																			{/each}
																		</div>
																	</svelte:fragment>
																</Popover>
															{:else}
																No repositories found
															{/if}
														</td>
														<td class="px-6 py-2 text-sm text-gray-900">
															<button
																class="rounded-full p-1.5 text-gray-500 hover:bg-gray-100 hover:text-gray-700"
																on:click={() => {
																	const installations = { ...$values['git_installations'] }
																	installations[workspace] = installations[workspace].filter(
																		(i) =>
																			i.installation_id !== installation.installation_id ||
																			i.account_id !== installation.account_id
																	)
																	if (installations[workspace].length === 0) {
																		delete installations[workspace]
																	}
																	$values['git_installations'] = installations
																}}
															>
																<X size={14} />
															</button>
														</td>
													</tr>
												{/each}
											{/each}
										</tbody>
									</table>
								</div>
							</div>
						</div>
					{/if}
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
		</label>
	{/if}
{/if}
