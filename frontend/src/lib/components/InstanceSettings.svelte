<script lang="ts">
	import { settings, settingsKeys, type SettingStorage } from './instanceSettings'
	import { Button, Skeleton, Tab, TabContent, Tabs } from '$lib/components/common'
	import { SettingService, SettingsService } from '$lib/gen'
	import Toggle from '$lib/components/Toggle.svelte'
	import SecondsInput from '$lib/components/common/seconds/SecondsInput.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import OAuthSetting from '$lib/components/OAuthSetting.svelte'
	import { deepEqual } from 'fast-equals'
	import OktaSetting from './OktaSetting.svelte'
	import CloseButton from './common/CloseButton.svelte'
	import KeycloakSetting from './KeycloakSetting.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { capitalize, classNames, sleep } from '$lib/utils'
	import { enterpriseLicense, isCriticalAlertsUIOpen } from '$lib/stores'
	import CustomOauth from './CustomOauth.svelte'
	import {
		AlertCircle,
		AlertTriangle,
		BadgeCheck,
		Info,
		Plus,
		X,
		BadgeX,
		Slack
	} from 'lucide-svelte'
	import CustomSso from './CustomSso.svelte'
	import AuthentikSetting from '$lib/components/AuthentikSetting.svelte'
	import AutheliaSetting from '$lib/components/AutheliaSetting.svelte'
	import KanidmSetting from '$lib/components/KanidmSetting.svelte'
	import ZitadelSetting from '$lib/components/ZitadelSetting.svelte'
	import Password from './Password.svelte'
	import ObjectStoreConfigSettings from './ObjectStoreConfigSettings.svelte'
	import { fade } from 'svelte/transition'
	import Popover from './Popover.svelte'

	import { base } from '$lib/base'
	import { createEventDispatcher } from 'svelte'
	import { setLicense } from '$lib/enterpriseUtils'

	export let tab: string = 'Core'
	export let hideTabs: boolean = false
	export let hideSave: boolean = false
	export let closeDrawer: (() => void) | undefined = () => {}

	let values: Record<string, any> = {}
	let initialOauths: Record<string, any> = {}
	let initialRequirePreexistingUserForOauth: boolean = false
	let requirePreexistingUserForOauth: boolean = false
	let ssoOrOauth: 'sso' | 'oauth' = 'sso'
	let latestKeyRenewalAttempt: {
		result: string
		attempted_at: string
	} | null = null

	let initialValues: Record<string, any> = {}
	let loading = true

	let version: string = ''

	loadSettings()
	loadVersion()

	const dispatch = createEventDispatcher()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}

	async function loadSettings() {
		loading = true

		async function getValue(key: string, storage: SettingStorage) {
			if (storage == 'setting') {
				return SettingService.getGlobal({ key })
			}
		}
		initialOauths = (await SettingService.getGlobal({ key: 'oauths' })) ?? {}
		requirePreexistingUserForOauth =
			((await SettingService.getGlobal({ key: 'require_preexisting_user_for_oauth' })) as any) ??
			false
		initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
		oauths = JSON.parse(JSON.stringify(initialOauths))
		initialValues = Object.fromEntries(
			(
				await Promise.all(
					Object.entries(settings).map(
						async ([_, y]) =>
							await Promise.all(y.map(async (x) => [x.key, await getValue(x.key, x.storage)]))
					)
				)
			).flat()
		)
		values = JSON.parse(JSON.stringify(initialValues))
		if (values['base_url'] == undefined) {
			values['base_url'] = window.location.origin
		}
		if (values['retention_period_secs'] == undefined) {
			values['retention_period_secs'] = 60 * 60 * 24 * 30
		}
		if (values['base_url'] == undefined) {
			values['base_url'] = 'http://localhost'
		}
		if (values['smtp_settings'] == undefined) {
			values['smtp_settings'] = {}
		}
		if (values['indexer_settings'] == undefined) {
			values['indexer_settings'] = {}
		}
		loading = false

		latestKeyRenewalAttempt = await SettingService.getLatestKeyRenewalAttempt()

		// populate snowflake account identifier from db
		const account_identifier =
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier
		if (account_identifier) {
			snowflakeAccountIdentifier = account_identifier
		}
	}

	export async function saveSettings() {
		if (
			oauths?.snowflake_oauth &&
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier !==
				snowflakeAccountIdentifier
		) {
			setupSnowflakeUrls()
		}

		let shouldReloadPage = false
		if (values) {
			const allSettings = Object.values(settings).flatMap((x) => Object.entries(x))
			let licenseKeySet = false
			await Promise.all(
				allSettings
					.filter((x) => {
						return (
							x[1].storage == 'setting' &&
							!deepEqual(initialValues?.[x[1].key], values?.[x[1].key]) &&
							(values?.[x[1].key] != '' ||
								initialValues?.[x[1].key] != undefined ||
								initialValues?.[x[1].key] != null)
						)
					})
					.map(async ([_, x]) => {
						if (x.key == 'license_key') {
							licenseKeySet = true
						}
						if (x.requiresReloadOnChange) {
							shouldReloadPage = true
						}
						return await SettingService.setGlobal({
							key: x.key,
							requestBody: { value: values?.[x.key] }
						})
					})
			)
			initialValues = JSON.parse(JSON.stringify(values))

			if (!deepEqual(initialOauths, oauths)) {
				await SettingService.setGlobal({
					key: 'oauths',
					requestBody: {
						value: oauths
					}
				})
				initialOauths = JSON.parse(JSON.stringify(oauths))
			}
			if (initialRequirePreexistingUserForOauth !== requirePreexistingUserForOauth) {
				await SettingService.setGlobal({
					key: 'require_preexisting_user_for_oauth',
					requestBody: { value: requirePreexistingUserForOauth }
				})
			}
			if (licenseKeySet) {
				setLicense()
			}
		} else {
			console.error('Values not loaded')
		}
		if (shouldReloadPage) {
			sendUserToast('Settings updated, reloading page...')
			await sleep(1000)
			window.location.reload()
		} else {
			sendUserToast('Settings updated')
			dispatch('saved')
		}
	}

	let oauths: Record<string, any> = {}

	let resourceName = ''

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

	let to: string = ''

	const windmillBuiltins = [
		'github',
		'gitlab',
		'bitbucket',
		'slack',
		'gsheets',
		'gdrive',
		'gmail',
		'gcal',
		'gforms',
		'gcloud',
		'gworkspace',
		'basecamp',
		'linkedin',
		'quickbooks',
		'visma',
		'spotify',
		'snowflake_oauth'
	]

	let oauth_name = undefined

	async function sendStats() {
		await SettingService.sendStats()
		sendUserToast('Usage sent')
	}

	let clientName = ''

	let licenseKeyChanged = false

	let renewing = false
	export async function renewLicenseKey() {
		renewing = true
		try {
			await SettingService.renewLicenseKey({
				licenseKey: values['license_key'] || undefined
			})
			sendUserToast('Key renewal successful')
			loadSettings()
		} catch (err) {
			latestKeyRenewalAttempt = await SettingService.getLatestKeyRenewalAttempt()
			throw err
		} finally {
			renewing = false
		}
	}

	let opening = false
	export async function openCustomerPortal() {
		opening = true
		try {
			const url = await SettingService.createCustomerPortalSession({
				licenseKey: values['license_key'] || undefined
			})
			window.open(url, '_blank')
		} finally {
			opening = false
		}
	}

	function showSetting(setting: string, values: Record<string, any>) {
		if (setting == 'dev_instance') {
			if (values['license_key'] == undefined) {
				return false
			}
		}
		return true
	}

	let snowflakeAccountIdentifier = ''

	function setupSnowflakeUrls() {
		// strip all whitespaces from account identifier
		snowflakeAccountIdentifier = snowflakeAccountIdentifier.replace(/\s/g, '')

		const connect_config = {
			scopes: [],
			auth_url: `https://${snowflakeAccountIdentifier}.snowflakecomputing.com/oauth/authorize`,
			token_url: `https://${snowflakeAccountIdentifier}.snowflakecomputing.com/oauth/token-request`,
			req_body_auth: false,
			extra_params: { account_identifier: snowflakeAccountIdentifier },
			extra_params_callback: {}
		}
		oauths['snowflake_oauth'].connect_config = connect_config
	}
</script>

<div class="pb-8">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<Tabs {hideTabs} bind:selected={tab}>
		{#each settingsKeys as category}
			<Tab value={category}>{category}</Tab>
		{/each}

		<svelte:fragment slot="content">
			<div class="pt-4" />

			{#each Object.keys(settings) as category}
				<TabContent value={category}>
					{#if category == 'SMTP'}
						<div class="text-secondary pb-4 text-xs"
							>Setting SMTP unlocks sending emails upon adding new users to the workspace or the
							instance or sending critical alerts.
							<a target="_blank" href="https://www.windmill.dev/docs/misc/setup_smtp">Learn more</a
							></div
						>
					{:else if category == "Indexer/Search"}
						<div class="text-secondary pb-4 text-xs"
							>The indexer service unlocks full text search across jobs and service logs. It requires spinning up its own separate container
							<a target="_blank" href="https://www.windmill.dev/docs/core_concepts/search_bar#setup">Learn how to</a
							></div
						>
					{:else if category == 'Registries'}
						<div class="text-secondary pb-4 text-xs">
							Add private registries for Pip, Bun and npm. <a
								target="_blank"
								href="https://www.windmill.dev/docs/advanced/imports">Learn more</a
							>
						</div>
					{:else if category == 'Slack'}
						<div class="text-secondary pb-4 text-xs">
							Connecting your instance to a Slack workspace enables critical alerts to be sent to a
							Slack channel.
							<a target="_blank" href="https://www.windmill.dev/docs/misc/saml_and_scim"
								>Learn more</a
							>
						</div>
					{:else if category == 'SCIM/SAML'}
						<div class="text-secondary pb-4 text-xs">
							Setting up SAML and SCIM allows you to authenticate users using your identity
							provider.
							<a
								target="_blank"
								href="https://www.windmill.dev/docs/advanced/instance_settings#slack">Learn more</a
							>
						</div>
					{:else if category == 'Debug'}
						<div class="text-secondary pb-4 text-xs">
							Enable debug mode to get more detailed logs.
						</div>
					{:else if category == 'Telemetry'}
						<div class="text-secondary pb-4 text-xs">
							Anonymous usage data is collected to help improve Windmill.
							<br />The following information is collected:
							<ul class="list-disc list-inside pl-2">
								<li>version of your instance</li>
								<li>number and total duration of jobs</li>
								<li>accounts usage</li>
								<li>login type usage</li>
								<li>workers usage</li>
								<li>vCPUs usage</li>
								<li>memory usage</li>
							</ul>
						</div>
						{#if $enterpriseLicense}
							<div class="text-secondary pb-4 text-xs">
								On Enterprise Edition, you must send data to check that usage is in line with the
								terms of the subscription. You can either enable telemetry or regularly send usage
								data by clicking the button below.
							</div>
							<Button
								on:click={sendStats}
								variant="border"
								color="light"
								btnClasses="w-auto"
								wrapperClasses="mb-4"
								size="xs">Send usage</Button
							>
						{/if}
					{:else if category == 'SSO/OAuth'}
						<div>
							<Tabs bind:selected={ssoOrOauth} class="mt-2 mb-4">
								<Tab value="sso">SSO</Tab>
								<Tab value="oauth">OAuth</Tab>
							</Tabs>
						</div>

						<div class="mb-6">
							{#if ssoOrOauth === 'sso'}
								{#if !$enterpriseLicense || $enterpriseLicense.endsWith('_pro')}
									<Alert type="warning" title="Limited to 10 SSO users">
										Without EE, the number of SSO users is limited to 10. SCIM/SAML is available on
										EE
									</Alert>
								{/if}

								<div class="py-1" />
								<div class="mb-2">
									<span class="text-primary text-sm"
										>When at least one of the below options is set, users will be able to login to
										Windmill via their third-party account.
										<br /> To test SSO, the recommended workflow is to to save the settings and try
										to login in an incognito window.
										<a target="_blank" href="https://www.windmill.dev/docs/misc/setup_oauth#sso"
											>Learn more</a
										></span
									>
								</div>
								<div class="flex flex-col gap-2 py-4">
									<OAuthSetting name="google" bind:value={oauths['google']} />
									<OAuthSetting name="microsoft" bind:value={oauths['microsoft']} />
									<OktaSetting bind:value={oauths['okta']} />
									<OAuthSetting name="github" bind:value={oauths['github']} />
									<OAuthSetting name="gitlab" bind:value={oauths['gitlab']} />
									<OAuthSetting name="jumpcloud" bind:value={oauths['jumpcloud']} />
									<KeycloakSetting bind:value={oauths['keycloak']} />
									<AuthentikSetting bind:value={oauths['authentik']} />
									<AutheliaSetting bind:value={oauths['authelia']} />
									<KanidmSetting bind:value={oauths['kanidm']} />
									<ZitadelSetting bind:value={oauths['zitadel']} />
									{#each Object.keys(oauths) as k}
										{#if !['authelia', 'authentik', 'google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'keycloak', 'slack', 'kanidm', 'zitadel'].includes(k) && 'login_config' in oauths[k]}
											{#if oauths[k]}
												<div class="flex flex-col gap-2 pb-4">
													<div class="flex flex-row items-center gap-2">
														<label class="text-md font-medium text-primary">{k}</label>
														<CloseButton
															on:close={() => {
																delete oauths[k]
																oauths = { ...oauths }
															}}
														/>
													</div>
													<div class="p-2 border rounded">
														<label class="block pb-2">
															<span class="text-primary font-semibold text-sm">Custom Name</span>
															<input
																type="text"
																placeholder="Custom Name"
																bind:value={oauths[k]['display_name']}
															/>
														</label>
														<label class="block pb-2">
															<span class="text-primary font-semibold text-sm">Client Id</span>
															<input
																type="text"
																placeholder="Client Id"
																bind:value={oauths[k]['id']}
															/>
														</label>
														<label class="block pb-2">
															<span class="text-primary font-semibold text-sm">Client Secret</span>
															<input
																type="text"
																placeholder="Client Secret"
																bind:value={oauths[k]['secret']}
															/>
														</label>
														{#if !windmillBuiltins.includes(k) && k != 'slack'}
															<CustomSso bind:login_config={oauths[k]['login_config']} />
														{/if}
													</div>
												</div>
											{/if}
										{/if}
									{/each}
								</div>
								<div class="flex gap-2 py-4">
									<input type="text" placeholder="client_id" bind:value={clientName} />
									<Button
										variant="border"
										color="blue"
										hover="yo"
										size="sm"
										endIcon={{ icon: Plus }}
										disabled={clientName == ''}
										on:click={() => {
											oauths[clientName] = { id: '', secret: '', login_config: {} }
											clientName = ''
										}}
									>
										Add custom SSO client {!$enterpriseLicense ? '(requires ee)' : ''}
									</Button>
								</div>
								<div class="flex gap-2 py-4">
									<Toggle
										options={{
											right:
												'Require users to have been added manually to Windmill to sign in through OAuth'
										}}
										bind:checked={requirePreexistingUserForOauth}
									/>
								</div>
							{:else if ssoOrOauth === 'oauth'}
								<div class="mb-2">
									<span class="text-primary text-sm"
										>When one of the below options is set, you will be able to create a specific
										resource containing a token automatically generated by the third-party provider.
										<br />
										To test it after setting an oauth client, go to the Resources menu and create a new
										one of the type of your oauth client (i.e. a 'github' resource if you set Github
										OAuth).
										<br /><a
											target="_blank"
											href="https://www.windmill.dev/docs/misc/setup_oauth#oauth">Learn more</a
										></span
									>
								</div>
								<div class="py-1" />
								<OAuthSetting login={false} name="slack" bind:value={oauths['slack']} />
								<div class="py-1" />

								{#each Object.keys(oauths) as k}
									{#if oauths[k] && !('login_config' in oauths[k])}
										{#if !['slack'].includes(k) && oauths[k]}
											<div class="flex flex-col gap-2 pb-4">
												<div class="flex flex-row items-center gap-2">
													<label class="text-md font-medium text-primary">{k}</label>
													<CloseButton
														on:close={() => {
															delete oauths[k]
															oauths = { ...oauths }
														}}
													/>
												</div>
												<div class="p-2 border rounded">
													<label class="block pb-2">
														<span class="text-primary font-semibold text-sm">Client Id</span>
														<input
															type="text"
															placeholder="Client Id"
															bind:value={oauths[k]['id']}
														/>
													</label>
													<label class="block pb-2">
														<span class="text-primary font-semibold text-sm">Client Secret</span>
														<input
															type="text"
															placeholder="Client Secret"
															bind:value={oauths[k]['secret']}
														/>
													</label>
													{#if !windmillBuiltins.includes(k) && k != 'slack'}
														<CustomOauth bind:connect_config={oauths[k]['connect_config']} />
													{/if}
													{#if k == 'snowflake_oauth'}
														<label class="block pb-2">
															<span class="text-primary font-semibold text-sm"
																><a
																	href="https://docs.snowflake.com/en/user-guide/admin-account-identifier#using-an-account-name-as-an-identifier"
																	target="_blank">Snowflake Account Identifier</a
																></span
															>
															<input
																type="text"
																placeholder="<orgname>-<account_name>"
																required={true}
																bind:value={snowflakeAccountIdentifier}
															/>
														</label>
													{/if}
												</div>
											</div>
										{/if}
									{/if}
								{/each}

								<div class="flex gap-2">
									<select name="oauth_name" id="oauth_name" bind:value={oauth_name}>
										<option value={undefined}>Select an OAuth client</option>
										<option value="custom">Fully Custom (requires ee)</option>
										{#each windmillBuiltins as name}
											<option value={name}>{capitalize(name)}</option>
										{/each}
									</select>
									{#if oauth_name == 'custom'}
										<input type="text" placeholder="client_id" bind:value={resourceName} />
									{:else}
										<input type="text" value={oauth_name ?? ''} disabled />
									{/if}
									<Button
										variant="border"
										color="blue"
										hover="yo"
										size="sm"
										endIcon={{ icon: Plus }}
										disabled={!oauth_name ||
											(oauth_name == 'custom' && resourceName == '') ||
											(oauth_name == 'custom' && !$enterpriseLicense)}
										on:click={() => {
											let name = oauth_name == 'custom' ? resourceName : oauth_name
											oauths[name ?? ''] = { id: '', secret: '' }
											resourceName = ''
										}}
									>
										Add OAuth client {oauth_name == 'custom' && !$enterpriseLicense
											? '(requires ee)'
											: ''}
									</Button>
								</div>
							{/if}
						</div>
					{/if}
					<div>
						<div class="flex-col flex gap-2 pb-4">
							{#each settings[category] as setting}
								{#if (!setting.cloudonly || isCloudHosted()) && showSetting(setting.key, values) && !(setting.hiddenIfNull && values[setting.key] == null)}
									{#if setting.ee_only != undefined && !$enterpriseLicense}
										<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap">
											<AlertTriangle size={16} />
											EE only {#if setting.ee_only != ''}<Tooltip>{setting.ee_only}</Tooltip>{/if}
										</div>
									{/if}
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
										{#if values}
											{@const hasError = setting.isValid && !setting.isValid(values[setting.key])}
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
													bind:value={values[setting.key]}
												/>
												{#if setting.advancedToggle}
													<div class="mt-1">
														<Toggle
															size="xs"
															options={{ right: setting.advancedToggle.label }}
															checked={setting.advancedToggle.checked(values)}
															on:change={() => {
																if (setting.advancedToggle) {
																	values = setting.advancedToggle.onChange(values)
																}
															}}
														/>
													</div>
												{/if}
											{:else if setting.fieldType == 'textarea'}
												<textarea
													rows="2"
													placeholder={setting.placeholder}
													bind:value={values[setting.key]}
												/>
												{#if setting.key == 'saml_metadata'}
													<div class="flex mt-2">
														<Button
															on:click={async (e) => {
																const res = await SettingService.testMetadata({
																	requestBody: values[setting.key]
																})
																sendUserToast(`Metadata valid, see console for full content`)
																console.log(`Metadata content:`, res)
															}}>Test content/url</Button
														>
													</div>
												{/if}
											{:else if setting.fieldType == 'license_key'}
												{@const { valid, expiration } = parseLicenseKey(values[setting.key] ?? '')}
												<div class="flex gap-2">
													<Password
														small
														placeholder={setting.placeholder}
														on:keydown={() => {
															licenseKeyChanged = true
														}}
														bind:password={values[setting.key]}
													/>
													<Button
														variant={values[setting.key] ? 'contained' : 'border'}
														size="xs"
														on:click={async () => {
															await SettingService.testLicenseKey({
																requestBody: { license_key: values[setting.key] }
															})
															sendUserToast('Valid key')
														}}
													>
														Test Key
													</Button>
												</div>
												<div class="mt-1 flex flex-col gap-1 items-start">
													{#if values[setting.key]?.length > 0}
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
														{@const attemptedAt = new Date(
															latestKeyRenewalAttempt.attempted_at
														).toLocaleString()}
														{@const isTrial =
															latestKeyRenewalAttempt.result.startsWith('error: trial:')}
														<div class="relative">
															<Popover notClickable>
																<div class="flex flex-row items-center gap-1">
																	{#if latestKeyRenewalAttempt.result === 'success'}
																		<BadgeCheck class="text-green-600" size={12} />
																	{:else}
																		<BadgeX
																			class={isTrial ? 'text-yellow-600' : 'text-red-600'}
																			size={12}
																		/>
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
																	As long as invoices are paid and usage corresponds to the subscription,
																	the key is renewed daily with a validity of 35 days (grace period).
																</div>
															</Popover>
														</div>
													{/if}
													{#if licenseKeyChanged && !$enterpriseLicense}
														{#if version.startsWith('CE')}
															<div class="text-red-400"
																>License key is set but image used is the Community Edition {version}.
																Switch image to EE.</div
															>
														{/if}
													{/if}

													{#if valid || expiration}
														<div class="flex flex-row gap-2 mt-1">
															<Button
																on:click={renewLicenseKey}
																loading={renewing}
																size="xs"
																color="dark"
																>Renew key
															</Button>
															<Button
																color="dark"
																size="xs"
																loading={opening}
																on:click={openCustomerPortal}
															>
																Open customer portal
															</Button>
														</div>
													{/if}
												</div>
											{:else if setting.fieldType == 'email'}
												<input
													type="email"
													placeholder={setting.placeholder}
													bind:value={values[setting.key]}
												/>
											{:else if setting.key == 'critical_alert_mute_ui'}
												<div class="flex flex-col gap-y-2 my-2 py-2">
													<Toggle
														disabled={!$enterpriseLicense}
														bind:checked={values[setting.key]}
														options={{ right: setting.description }}
													/>
													<div class="flex flex-row">
														<Button
															disabled={!$enterpriseLicense}
															size="sm"
															on:click={() => {
																isCriticalAlertsUIOpen.set(true)
																closeDrawer?.()
															}}
														>
															Show Critical Alerts
														</Button>
													</div>
												</div>
											{:else if setting.fieldType == 'critical_error_channels'}
												<div class="w-full flex gap-x-16 flex-wrap">
													<div class="w-full max-w-lg">
														<div class="flex w-full max-w-lg mt-1 gap-2 w-full items-center">
															<input
																type="text"
																placeholder="Logs (critical errors are always logged)"
																disabled
															/>
														</div>

														{#if $enterpriseLicense && Array.isArray(values[setting.key])}
															{#each values[setting.key] ?? [] as v, i}
																<div class="flex w-full max-w-lg mt-1 gap-2 w-full items-center">
																	<select
																		class="w-20"
																		on:change={(e) => {
																			if (e.target?.['value']) {
																				values[setting.key][i] = {
																					[e.target['value']]: ''
																				}
																			}
																		}}
																		value={v && 'slack_channel' in v ? 'slack_channel' : 'email'}
																	>
																		<option value="email">Email</option>
																		<option value="slack_channel">Slack</option>
																	</select>
																	{#if v && 'slack_channel' in v}
																		<input
																			type="text"
																			placeholder="Slack channel"
																			on:input={(e) => {
																				if (e.target?.['value']) {
																					values[setting.key][i] = {
																						slack_channel: e.target['value']
																					}
																				}
																			}}
																			value={v?.slack_channel ?? ''}
																		/>
																	{:else}
																		<input
																			type="email"
																			placeholder="Email address"
																			on:input={(e) => {
																				if (e.target?.['value']) {
																					values[setting.key][i] = {
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
																			values[setting.key] = values[setting.key].filter(
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
													<div
														><div class="flex mt-1">
															<Button
																size="xs"
																variant="contained"
																on:click={async () => {
																	try {
																		await SettingService.testCriticalChannels({
																			requestBody: values[setting.key]
																		})
																		sendUserToast(
																			'Test message sent successfully to critical channels',
																			false
																		)
																	} catch (error) {
																		sendUserToast(
																			'Failed to send test message: ' + error.message,
																			true
																		)
																	}
																}}
															>
																Test Critical Channels
															</Button>
														</div>
													</div>
												</div>
												<div class="flex mt-2 gap-20 items-baseline">
													<Button
														variant="border"
														color="light"
														size="xs"
														btnClasses="mt-1"
														on:click={() => {
															if (
																values[setting.key] == undefined ||
																!Array.isArray(values[setting.key])
															) {
																values[setting.key] = []
															}
															values[setting.key] = values[setting.key].concat('')
														}}
														id="arg-input-add-item"
														startIcon={{ icon: Plus }}
														disabled={!$enterpriseLicense}
													>
														Add item
													</Button>
												</div>
											{:else if setting.fieldType == 'slack_connect'}
												<div class="flex flex-col items-start self-start">
													{#if values[setting.key] && 'team_name' in values[setting.key]}
														<div class="text-sm">
															Connected to <code>{values[setting.key]['team_name']}</code>
														</div>
														<Button
															size="sm"
															endIcon={{ icon: Slack }}
															btnClasses="mt-2"
															variant="border"
															on:click={async () => {
																values[setting.key] = undefined
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
													{#if values[setting.key]}
														<div>
															<label for="writer_memory_budget" class="block text-sm font-medium">
																Index writer memory budget (MB)
																<Tooltip>
																	The allocated memory arena for the indexer. A bigger value means
																	less writing to disk and potentially higher indexing throughput
																</Tooltip>
															</label>
															<input
																type="number"
																id="writer_memory_budget"
																placeholder="300"
																on:input={(e) => {
																	if (e.target instanceof HTMLInputElement) {
																		if (e.target.valueAsNumber) {
																			values[setting.key].writer_memory_budget =
																				e.target.valueAsNumber * (1024 * 1024)
																		}
																	}
																}}
																value={values[setting.key].writer_memory_budget / (1024 * 1024)}
															/>
														</div>
														<h3>Completed Job Index</h3>
														<div>
															<label
																for="commit_job_max_batch_size"
																class="block text-sm font-medium"
															>
																Commit max batch size <Tooltip>
																	The max amount of documents (here jobs) per commit. To optimize
																	indexing throughput, it is best to keep this as high as possible.
																	However, especially when reindexing the whole instance, it can be
																	useful to have a limit on how many jobs can be written without
																	being commited. A commit will make the jobs available for search,
																	constitute a "checkpoint" state in the indexing and will be
																	logged.
																</Tooltip>
															</label>
															<input
																type="number"
																id="commit_job_max_batch_size"
																placeholder="100000"
																bind:value={values[setting.key].commit_job_max_batch_size}
															/>
														</div>
														<div>
															<label for="refresh_index_period" class="block text-sm font-medium">
																Refresh index period (s) <Tooltip>
																	The index will query new jobs peridically and write them on the
																	index. This setting sets that period.
																</Tooltip></label
															>
															<input
																type="number"
																id="refresh_index_period"
																placeholder="300"
																bind:value={values[setting.key].refresh_index_period}
															/>
														</div>
														<div>
															<label
																for="max_indexed_job_log_size"
																class="block text-sm font-medium"
															>
																Max indexed job log size (KB) <Tooltip>
																	Job logs are included when indexing, but to avoid the index size
																	growing artificially, the logs will be truncated after a size has
																	been reached.
																</Tooltip>
															</label>
															<input
																type="number"
																id="max_indexed_job_log_size"
																placeholder="1024"
																on:input={(e) => {
																	if (e.target instanceof HTMLInputElement) {
																		if (e.target.valueAsNumber) {
																			values[setting.key].max_indexed_job_log_size =
																				e.target.valueAsNumber * 1024
																		}
																	}
																}}
																value={values[setting.key].max_indexed_job_log_size / 1024}
															/>
														</div>
														<h3>Service Logs Index</h3>
														<div>
															<label
																for="commit_log_max_batch_size"
																class="block text-sm font-medium"
																>Commit max batch size Commit max batch size <Tooltip>
																	The max amount of documents per commit. In this case 1 document is
																	one log file representing all logs during 1 minute for a specific
																	host. To optimize indexing throughput, it is best to keep this as
																	high as possible. However, especially when reindexing the whole
																	instance, it can be useful to have a limit on how many logs can be
																	written without being commited. A commit will make the logs
																	available for search, appear as a log line, and be a "checkpoint"
																	of the indexing progress.
																</Tooltip>
															</label>
															<input
																type="number"
																id="commit_log_max_batch_size"
																placeholder="10000"
																bind:value={values[setting.key].commit_log_max_batch_size}
															/>
														</div>
														<div>
															<label
																for="refresh_log_index_period"
																class="block text-sm font-medium"
															>
																Refresh index period (s) <Tooltip>
																	The index will query new service logs peridically and write them
																	on the index. This setting sets that period.
																</Tooltip></label
															>
															<input
																type="number"
																id="refresh_log_index_period"
																placeholder="300"
																bind:value={values[setting.key].refresh_log_index_period}
															/>
														</div>
													{/if}
												</div>
											{:else if setting.fieldType == 'smtp_connect'}
												<div class="flex flex-col gap-4 mt-4">
													{#if values[setting.key]}
														<div>
															<label for="smtp_host" class="block text-sm font-medium">Host</label>
															<input
																type="text"
																id="smtp_host"
																placeholder="smtp.gmail.com"
																bind:value={values[setting.key].smtp_host}
															/>
														</div>
														<div>
															<label for="smtp_port" class="block text-sm font-medium">Port</label>
															<input
																type="number"
																id="smtp_port"
																placeholder="587"
																bind:value={values[setting.key].smtp_port}
															/>
														</div>
														<div>
															<label for="smtp_username" class="block text-sm font-medium"
																>Username</label
															>
															<input
																type="text"
																id="smtp_username"
																placeholder="ruben@windmill.dev"
																bind:value={values[setting.key].smtp_username}
															/>
														</div>
														<div>
															<label for="smtp_password" class="block text-sm font-medium"
																>Password</label
															>
															<Password bind:password={values[setting.key].smtp_password} />
														</div>
														<div>
															<label for="smtp_from" class="block text-sm font-medium"
																>From Address</label
															>
															<input
																type="email"
																id="smtp_from"
																placeholder="noreply@windmill.dev"
																bind:value={values[setting.key].smtp_from}
															/>
														</div>
														<div>
															<Toggle
																disabled={values[setting.key].smtp_disable_tls}
																id="smtp_tls_implicit"
																bind:checked={values[setting.key].smtp_tls_implicit}
																options={{ right: 'Implicit TLS' }}
																label="Implicit TLS"
															/>
														</div>
														<div>
															<Toggle
																id="smtp_disable_tls"
																bind:checked={values[setting.key].smtp_disable_tls}
																on:change={() => {
																	if (values[setting.key].smtp_disable_tls) {
																		values[setting.key].smtp_tls_implicit = false
																	}
																}}
																options={{ right: 'Disable TLS' }}
																label="Disable TLS"
															/>
														</div>
													{/if}
												</div>
											{:else if setting.fieldType == 'object_store_config'}
												<ObjectStoreConfigSettings bind:bucket_config={values[setting.key]} />
												<div class="mb-6" />
											{:else if setting.fieldType == 'number'}
												<input
													type="number"
													placeholder={setting.placeholder}
													bind:value={values[setting.key]}
												/>
											{:else if setting.fieldType == 'password'}
												<input
													autocomplete="new-password"
													type="password"
													placeholder={setting.placeholder}
													bind:value={values[setting.key]}
												/>
											{:else if setting.fieldType == 'boolean'}
												<div>
													<Toggle bind:checked={values[setting.key]} />
												</div>
											{:else if setting.fieldType == 'seconds'}
												<div>
													<SecondsInput
														max={setting.ee_only != undefined && !$enterpriseLicense
															? 60 * 60 * 24 * 30
															: undefined}
														bind:seconds={values[setting.key]}
													/>
												</div>
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
							{/each}
						</div>
					</div>
					{#if category == 'SMTP'}
						{@const smtp = values['smtp_settings']}
						<div class="flex gap-4"
							><input type="email" bind:value={to} placeholder="contact@windmill.dev" />
							<Button
								disabled={to == '' || !smtp}
								on:click={async () => {
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
					{/if}
				</TabContent>
			{/each}
		</svelte:fragment>
	</Tabs>
</div>

{#if !hideSave}
	<Button on:click={saveSettings}>Save settings</Button>
	<div class="pb-8" />
{/if}
