<script lang="ts">
	import { settings, settingsKeys, type SettingStorage } from './instanceSettings'
	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import { ConfigService, SettingService } from '$lib/gen'
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
	import { capitalize } from '$lib/utils'
	import { enterpriseLicense } from '$lib/stores'
	import CustomOauth from './CustomOauth.svelte'
	import { AlertTriangle, Plus } from 'lucide-svelte'
	import CustomSso from './CustomSso.svelte'

	export let tab: string = 'Core'
	export let hideTabs: boolean = false

	let values: Record<string, any> = {}
	let initialOauths: Record<string, any> = {}

	let serverConfig = {}
	let initialValues: Record<string, any> = {}
	loadSettings()
	async function loadSettings() {
		try {
			serverConfig = (await ConfigService.getConfig({ name: 'server' })) ?? {}
		} catch (e) {
			console.log("Sever config not found, assuming it's first setup")
		}

		async function getValue(key: string, storage: SettingStorage) {
			if (storage == 'setting') {
				return SettingService.getGlobal({ key })
			} else if (storage == 'config') {
				return serverConfig[key]
			}
		}
		initialOauths = (await SettingService.getGlobal({ key: 'oauths' })) ?? {}
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
		if (values['retention_period_secs'] == undefined) {
			values['retention_period_secs'] = 60 * 60 * 24 * 60
		}
		if (values['base_url'] == undefined) {
			values['base_url'] = 'http://localhost'
		}
	}

	async function saveSettings() {
		if (values) {
			const allSettings = Object.values(settings).flatMap((x) => Object.entries(x))
			const newServerConfig = Object.fromEntries(
				allSettings
					.filter((x) => x[1].storage == 'config' && values?.[x[1].key] && values?.[x[1].key] != '')
					.map((x) => [x[1].key, values?.[x[1].key]])
			)
			if (!deepEqual(newServerConfig, serverConfig)) {
				await ConfigService.updateConfig({
					name: 'server',
					requestBody: newServerConfig
				})
				serverConfig = JSON.parse(JSON.stringify(newServerConfig))
			}
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
						await SettingService.setGlobal({ key: x.key, requestBody: { value: values?.[x.key] } })
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
		} else {
			console.error('Values not loaded')
		}
	}

	let oauths: Record<string, any> = {}

	let resourceName = ''

	function parseDate(license_key: string): string | undefined {
		let splitted = license_key.split('.')
		if (splitted.length >= 3) {
			try {
				let i = parseInt(splitted[1])
				let date = new Date(i * 1000)
				return date.toDateString()
			} catch {}
		}
		return undefined
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
		'gcloud',
		'gworkspace',
		'basecamp',
		'linkedin'
	]

	let oauth_name = 'custom'

	async function sendStats() {
		await SettingService.sendStats()
		sendUserToast('Usage sent')
	}

	let clientName = ''

	let licenseKeyChanged = false
</script>

<div class="pb-8">
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
							>Setting SMTP unlock sending emails upon adding new users to the workspace or the
							instance.</div
						>
					{:else if category == 'Telemetry'}
						<div class="text-secondary pb-4 text-xs">
							Anonymous usage data is collected to help improve Windmill.
							<br />The following information is collected:
							<ul class="list-disc list-inside pl-2">
								<li>version</li>
								<li>number and total duration of jobs</li>
								<li>login type usage</li>
								<li>workers usage</li>
							</ul>
						</div>
						{#if $enterpriseLicense}
							<Button
								on:click={sendStats}
								variant="border"
								color="light"
								btnClasses="w-auto"
								wrapperClasses="mb-4"
								size="xs">Send usage</Button
							>
						{/if}
					{/if}
					{#if category == 'SSO/OAuth'}
						<div>
							<h4 class="pb-4">SSO</h4>
							{#if !$enterpriseLicense}
								<Alert type="warning" title="Limited to 10 SSO users">
									Without EE, the number of SSO users is limited to 10. SCIM/SAML is available on EE
								</Alert>
							{/if}

							<div class="py-1" />
							<Alert type="info" title="Test on a separate tab">
								The recommended workflow is to to save your oauth setting and test them directly on
								the login or resource page
							</Alert>
							<div class="flex flex-col gap-2 py-4">
								<OAuthSetting name="google" bind:value={oauths['google']} />
								<OAuthSetting name="microsoft" bind:value={oauths['microsoft']} />
								<OktaSetting bind:value={oauths['okta']} />
								<OAuthSetting name="github" bind:value={oauths['github']} />
								<OAuthSetting name="gitlab" bind:value={oauths['gitlab']} />
								<OAuthSetting name="jumpcloud" bind:value={oauths['jumpcloud']} />
								<KeycloakSetting bind:value={oauths['keycloak']} />
								{#each Object.keys(oauths) as k}
									{#if !['google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'keycloak', 'slack'].includes(k) && 'login_config' in oauths[k]}
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
							<div class="flex gap-2">
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
									Add custom SSO client {!$enterpriseLicense ? '(require ee)' : ''}
								</Button>
							</div>
							<h4 class="py-4">OAuth</h4>
							<Alert type="info" title="Require a corresponding resource type">
								After setting an oauth client, make sure that there is a corresponding resource type
								with the same name with a "token" field in the admins workspace.
							</Alert>
							<div class="py-1" />
							<OAuthSetting login={false} name="slack" bind:value={oauths['slack']} />
							<div class="py-1" />

							{#each Object.keys(oauths) as k}
								{#if !['google', 'microsoft', 'github', 'gitlab', 'jumpcloud', 'okta', 'keycloak', 'slack'].includes(k) && !('login_config' in oauths[k])}
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
													<span class="text-primary font-semibold text-sm">Client Id</span>
													<input type="text" placeholder="Client Id" bind:value={oauths[k]['id']} />
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
											</div>
										</div>
									{/if}
								{/if}
							{/each}

							<div class="flex gap-2">
								<select name="oauth_name" id="oauth_name" bind:value={oauth_name}>
									<option value="custom">Fully Custom (require ee)</option>
									{#each windmillBuiltins as name}
										<option value={name}>{capitalize(name)}</option>
									{/each}
								</select>
								{#if oauth_name == 'custom'}
									<input type="text" placeholder="client_id" bind:value={resourceName} />
								{:else}
									<input type="text" value={oauth_name} disabled />
								{/if}
								<Button
									variant="border"
									color="blue"
									hover="yo"
									size="sm"
									endIcon={{ icon: Plus }}
									disabled={(oauth_name == 'custom' && resourceName == '') ||
										(oauth_name == 'custom' && !$enterpriseLicense)}
									on:click={() => {
										let name = oauth_name == 'custom' ? resourceName : oauth_name
										oauths[name] = { id: '', secret: '' }
										resourceName = ''
									}}
								>
									Add OAuth client {oauth_name == 'custom' && !$enterpriseLicense
										? '(require ee)'
										: ''}
								</Button>
							</div>
						</div>
					{:else}
						<div>
							<div class="flex-col flex gap-2 pb-4">
								{#each settings[category] as setting}
									{#if !setting.cloudonly || isCloudHosted()}
										{#if setting.ee_only != undefined && !$enterpriseLicense}
											<div
												class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap"
											>
												<AlertTriangle size={16} />
												EE only <Tooltip>{setting.ee_only}</Tooltip>
											</div>
										{/if}
										<label class="block pb-2">
											<span class="text-primary font-semibold text-sm">{setting.label}</span>
											{#if setting.description}
												<span class="text-secondary text-xs">{setting.description}</span>
											{/if}
											{#if setting.tooltip}
												<Tooltip>{setting.tooltip}</Tooltip>
											{/if}
											{#if values}
												{@const hasError = setting.isValid && !setting.isValid(values[setting.key])}
												{#if setting.fieldType == 'text'}
													<input
														disabled={setting.ee_only != undefined && !$enterpriseLicense}
														type="text"
														placeholder={setting.placeholder}
														class={hasError
															? 'border !border-red-700 !border-opacity-30 !focus:border-red-700 !focus:border-opacity-30 !bg-red-100'
															: ''}
														bind:value={values[setting.key]}
													/>
												{:else if setting.fieldType == 'textarea'}
													<textarea
														rows="2"
														placeholder={setting.placeholder}
														bind:value={values[setting.key]}
													/>
												{:else if setting.fieldType == 'license_key'}
													<div class="flex justify-between gap-2">
														<textarea
															rows="2"
															placeholder={setting.placeholder}
															on:keydown={() => {
																licenseKeyChanged = true
															}}
															bind:value={values[setting.key]}
														/>
														<Button
															variant={values[setting.key] ? 'contained' : 'border'}
															size="xs"
															on:click={async () => {
																await SettingService.testLicenseKey({
																	requestBody: { license_key: values[setting.key] }
																})
																sendUserToast('Valid key')
															}}>Test Key</Button
														>
													</div>
													{#if values[setting.key]?.length > 0}
														{#if parseDate(values[setting.key])}
															<span class="text-tertiary text-2xs"
																>License key expires on {parseDate(values[setting.key])}</span
															>
														{/if}
													{/if}
													{#if licenseKeyChanged}
														<div class="text-yellow-600"
															>Refresh page after setting license key and saving to unlock all
															features</div
														>
													{/if}
												{:else if setting.fieldType == 'email'}
													<input
														type="email"
														placeholder={setting.placeholder}
														bind:value={values[setting.key]}
													/>
												{:else if setting.fieldType == 'number'}
													<input
														type="number"
														placeholder={setting.placeholder}
														bind:value={values[setting.key]}
													/>
												{:else if setting.fieldType == 'password'}
													<input
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
														<SecondsInput bind:seconds={values[setting.key]} />
													</div>
												{/if}

												{#if hasError}
													<span class="text-red-500 text-xs">
														Base url must start with http:// or https:// and must not end with a
														trailing slash.
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
					{/if}
					{#if category == 'SMTP'}
						<div class="flex gap-4"
							><input type="email" bind:value={to} placeholder="contact@windmill.dev" />
							<Button
								disabled={to == ''}
								on:click={async () => {
									await SettingService.testSmtp({
										requestBody: {
											to,
											smtp: {
												host: values['smtp_host'],
												username: values['smtp_username'],
												password: values['smtp_password'],
												port: values['smtp_port'],
												from: values['smtp_from'],
												tls_implicit: values['smtp_tls_implicit']
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
<div class="py-4" />

<Button
	on:click={async () => {
		await saveSettings()
		sendUserToast('Settings updated')
	}}
>
	Save
</Button>
<div class="pb-8" />
