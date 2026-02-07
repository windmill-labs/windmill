<script lang="ts">
	import { scimSamlSetting, settings, settingsKeys } from './instanceSettings'
	import { Button, Tab, TabContent, Tabs } from '$lib/components/common'
	import { SettingService, SettingsService } from '$lib/gen'
	import type { TeamsChannel } from '$lib/gen/types.gen'

	import { sendUserToast } from '$lib/toast'
	import { deepEqual } from 'fast-equals'

	import { sleep } from '$lib/utils'
	import { enterpriseLicense } from '$lib/stores'

	import { createEventDispatcher } from 'svelte'
	import { setLicense } from '$lib/enterpriseUtils'
	import AuthSettings from './AuthSettings.svelte'
	import InstanceSetting from './InstanceSetting.svelte'
	import { writable, type Writable } from 'svelte/store'
	import { ExternalLink, Loader2 } from 'lucide-svelte'
	import YAML from 'yaml'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'

	interface Props {
		tab?: string
		hideTabs?: boolean
		hideSave?: boolean
		closeDrawer?: (() => void) | undefined
	}

	let {
		tab = $bindable('Core'),
		hideTabs = false,
		hideSave = false,
		closeDrawer = () => {}
	}: Props = $props()

	let values: Writable<Record<string, any>> = writable({})
	let initialOauths: Record<string, any> = {}
	let initialRequirePreexistingUserForOauth: boolean = false
	let requirePreexistingUserForOauth: boolean = $state(false)

	let initialValues: Record<string, any> = {}
	let snowflakeAccountIdentifier = $state('')
	let version: string = $state('')
	let loading = $state(true)

	loadSettings()
	loadVersion()

	const dispatch = createEventDispatcher()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}
	let oauths: Record<string, any> = $state({})

	async function loadSettings() {
		loading = true

		// Bulk-load all settings in a single API call
		const config = await SettingService.getInstanceConfig()
		const gs = (config.global_settings ?? {}) as Record<string, any>

		initialOauths = gs['oauths'] ?? {}
		requirePreexistingUserForOauth = gs['require_preexisting_user_for_oauth'] ?? false
		initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
		oauths = JSON.parse(JSON.stringify(initialOauths))

		// Build initialValues from the bulk response, keyed by setting name
		initialValues = {}
		for (const [key, value] of Object.entries(gs)) {
			initialValues[key] = value
		}

		let nvalues = JSON.parse(JSON.stringify(initialValues))
		if (nvalues['base_url'] == undefined) {
			nvalues['base_url'] = window.location.origin
		}
		if (nvalues['retention_period_secs'] == undefined) {
			nvalues['retention_period_secs'] = 60 * 60 * 24 * 30
		}
		if (nvalues['base_url'] == undefined) {
			nvalues['base_url'] = 'http://localhost'
		}
		if (nvalues['smtp_settings'] == undefined) {
			nvalues['smtp_settings'] = {}
		}
		if (nvalues['otel'] == undefined) {
			nvalues['otel'] = {}
		}
		if (nvalues['indexer_settings'] == undefined) {
			nvalues['indexer_settings'] = {}
		}

		if (nvalues['critical_error_channels'] == undefined) {
			nvalues['critical_error_channels'] = []
		}

		$values = nvalues
		loading = false

		// populate snowflake account identifier from db
		const account_identifier =
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier
		if (account_identifier) {
			snowflakeAccountIdentifier = account_identifier
		}
	}

	export async function saveSettings() {
		if (viewMode === 'yaml') {
			if (!syncYamlToForm()) {
				return
			}
		}

		if (
			oauths?.snowflake_oauth &&
			oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier !==
				snowflakeAccountIdentifier
		) {
			setupSnowflakeUrls()
		}

		// Remove empty or invalid entries for critical error channels
		$values.critical_error_channels = $values.critical_error_channels.filter((entry: any) => {
			if (!entry || typeof entry !== 'object') return false
			if ('teams_channel' in entry) {
				return isValidTeamsChannel(entry.teams_channel)
			}
			if ('slack_channel' in entry) {
				return typeof entry.slack_channel === 'string' && entry.slack_channel.trim() !== ''
			}
			if ('email' in entry) {
				return typeof entry.email === 'string' && entry.email.trim() !== ''
			}
			// Unknown shape
			return false
		})

		let shouldReloadPage = false
		if ($values) {
			// Trim license key before saving
			if ($values['license_key'] && typeof $values['license_key'] === 'string') {
				$values['license_key'] = $values['license_key'].trim()
			}

			// Check which settings require a page reload
			const allSettings = [...Object.values(settings), scimSamlSetting].flatMap((x) =>
				Object.entries(x)
			)
			let licenseKeySet = false
			for (const [_, x] of allSettings) {
				if (
					x.storage == 'setting' &&
					!deepEqual(initialValues?.[x.key], $values?.[x.key])
				) {
					if (x.key == 'license_key') {
						licenseKeySet = true
					}
					if (x.requiresReloadOnChange) {
						shouldReloadPage = true
					}
				}
			}

			// Build the full global_settings object for the bulk PUT
			const globalSettings: Record<string, any> = { ...$values }

			// Include oauths and require_preexisting_user_for_oauth
			if (!deepEqual(initialOauths, oauths)) {
				globalSettings['oauths'] = oauths
			}
			if (initialRequirePreexistingUserForOauth !== requirePreexistingUserForOauth) {
				globalSettings['require_preexisting_user_for_oauth'] = requirePreexistingUserForOauth
			}

			// Single bulk PUT — backend handles the diff
			await SettingService.setInstanceConfig({
				requestBody: {
					global_settings: globalSettings,
					worker_configs: {}
				}
			})

			initialValues = JSON.parse(JSON.stringify($values))
			initialOauths = JSON.parse(JSON.stringify(oauths))
			initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth

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

	let sendingStats = $state(false)
	async function sendStats() {
		try {
			sendingStats = true
			await SettingService.sendStats()
			sendUserToast('Usage sent')
		} catch (err) {
			throw err
		} finally {
			sendingStats = false
		}
	}

	function isValidTeamsChannel(value: any): value is TeamsChannel {
		return (
			typeof value === 'object' &&
			value !== null &&
			typeof value.team_id === 'string' &&
			value.team_id.trim() !== '' &&
			typeof value.team_name === 'string' &&
			value.team_name.trim() !== '' &&
			typeof value.channel_id === 'string' &&
			value.channel_id.trim() !== '' &&
			typeof value.channel_name === 'string' &&
			value.channel_name.trim() !== ''
		)
	}

	function openSmtpSettings() {
		tab = 'SMTP'
	}

	let viewMode: 'form' | 'yaml' = $state('form')
	let yamlCode = $state('')
	let yamlEditor: SimpleEditor | undefined = $state(undefined)
	let yamlError = $state('')

	function buildYamlObject(): Record<string, any> {
		const obj: Record<string, any> = { ...$values }
		if (oauths && Object.keys(oauths).length > 0) {
			obj['oauths'] = oauths
		}
		if (requirePreexistingUserForOauth) {
			obj['require_preexisting_user_for_oauth'] = requirePreexistingUserForOauth
		}
		return obj
	}

	function syncFormToYaml() {
		yamlCode = YAML.stringify(buildYamlObject())
		yamlEditor?.setCode(yamlCode)
		yamlError = ''
	}

	function syncYamlToForm(): boolean {
		try {
			const parsed = YAML.parse(yamlCode)
			if (typeof parsed !== 'object' || parsed === null) {
				sendUserToast('YAML must be a mapping (key: value)', true)
				return false
			}
			if ('oauths' in parsed) {
				oauths = parsed['oauths'] ?? {}
				delete parsed['oauths']
			}
			if ('require_preexisting_user_for_oauth' in parsed) {
				requirePreexistingUserForOauth = parsed['require_preexisting_user_for_oauth'] ?? false
				delete parsed['require_preexisting_user_for_oauth']
			}
			$values = parsed
			yamlError = ''
			return true
		} catch (e) {
			yamlError = String(e)
			sendUserToast('Invalid YAML: ' + e, true)
			return false
		}
	}

	function handleViewModeChange(newMode: string) {
		if (newMode === 'yaml') {
			syncFormToYaml()
			viewMode = 'yaml'
		} else if (newMode === 'form') {
			if (syncYamlToForm()) {
				viewMode = 'form'
			} else {
				// Reset toggle back to YAML on parse failure
				viewMode = 'yaml'
			}
		}
	}
</script>

<div class="pb-12">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	{#if !hideTabs}
		<div class="flex items-center justify-end mb-2">
			<ToggleButtonGroup
				bind:selected={viewMode}
				onSelected={handleViewModeChange}
			>
				{#snippet children({ item })}
					<ToggleButton value="form" label="Form" {item} small />
					<ToggleButton value="yaml" label="YAML" {item} small />
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{/if}

	{#if viewMode === 'yaml' && !hideTabs}
		<div class="border rounded w-full" style="min-height: 600px;">
			{#await import('$lib/components/SimpleEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					bind:this={yamlEditor}
					autoHeight
					lang="yaml"
					bind:code={yamlCode}
					fixedOverflowWidgets={false}
				/>
			{/await}
		</div>
		{#if yamlError}
			<div class="text-red-500 text-xs mt-1">{yamlError}</div>
		{/if}
	{:else if hideTabs}
		{@render tabsContent()}
	{:else}
		<Tabs bind:selected={tab}>
			{#each settingsKeys as category}
				<Tab value={category} label={category}></Tab>
			{/each}

			{#snippet content()}
				<div class="pt-4"></div>
				{@render tabsContent()}
			{/snippet}
		</Tabs>
	{/if}

	{#snippet tabsContent()}
		{#each Object.keys(settings) as category}
			<TabContent value={category}>
				{#if category == 'SMTP'}
					<div class="text-secondary pb-4 text-xs">
						Setting SMTP unlocks sending emails upon adding new users to the workspace or the
						instance or sending critical alerts via email.
						<a target="_blank" href="https://www.windmill.dev/docs/advanced/instance_settings#smtp"
							>Learn more <ExternalLink size={12} class="inline-block" /></a
						>
					</div>
				{:else if category == 'Indexer/Search'}
					<div class="text-secondary pb-4 text-xs"
						>The indexer service unlocks full text search across jobs and service logs. It requires
						spinning up its own separate container
						<a target="_blank" href="https://www.windmill.dev/docs/core_concepts/search_bar#setup"
							>Learn how to <ExternalLink size={12} class="inline-block" /></a
						></div
					>
				{:else if category == 'Alerts'}
					<div class="text-secondary pb-4 text-xs">
						Critical alerts automatically notify administrators about system events like job crashes,
						license issues, worker failures, and queue delays through email, Slack, or Teams.
						<a target="_blank" href="https://www.windmill.dev/docs/core_concepts/critical_alerts"
							>Learn more <ExternalLink size={12} class="inline-block" /></a
						>
					</div>
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
						<a target="_blank" href="https://www.windmill.dev/docs/misc/saml_and_scim">Learn more</a
						>
					</div>
				{:else if category == 'SCIM/SAML'}
					<div class="text-secondary pb-4 text-xs">
						Setting up SAML and SCIM allows you to authenticate users using your identity provider.
						<a target="_blank" href="https://www.windmill.dev/docs/advanced/instance_settings#slack"
							>Learn more</a
						>
					</div>
				{:else if category == 'Debug'}
					<div class="text-secondary pb-4 text-xs">
						Enable debug mode to get more detailed logs.
					</div>
				{:else if category == 'Telemetry'}
					<div class="text-primary pb-4 text-xs">
						Anonymous usage data is collected to help improve Windmill.
						<br />The following information is collected:
						<ul class="list-disc list-inside pl-2">
							<li>version of your instances</li>
							<li>instance base URL</li>
							<li>job usage (language, total duration, count)</li>
							<li>login type usage (login type, count)</li>
							<li>worker usage (worker, worker instance, vCPUs, memory)</li>
							<li>user usage (author count, operator count)</li>
							<li>superadmin email addresses</li>
							<li>vCPU usage</li>
							<li>memory usage</li>
							<li>development instance status</li>
						</ul>
					</div>
					{#if $enterpriseLicense}
						<div class="text-primary pb-4 text-xs">
							On Enterprise Edition, you must send data to check that usage is in line with the
							terms of the subscription. You can either enable telemetry or regularly send usage
							data by clicking the button below.
						</div>
						<Button
							on:click={sendStats}
							variant="default"
							btnClasses="w-auto"
							wrapperClasses="mb-4"
							loading={sendingStats}
							size="xs"
						>
							Send usage
						</Button>
					{/if}
				{:else if category == 'Auth/OAuth/SAML'}
					<AuthSettings
						bind:oauths
						bind:snowflakeAccountIdentifier
						bind:requirePreexistingUserForOauth
						baseUrl={$values?.base_url}
					>
						{#snippet scim()}
							<div class="flex-col flex gap-6 pb-4">
								{#each scimSamlSetting as setting}
									<InstanceSetting
										on:closeDrawer={() => closeDrawer?.()}
										{loading}
										{setting}
										{values}
										{version}
										{oauths}
									/>
								{/each}
							</div>
						{/snippet}
					</AuthSettings>
				{/if}

				<div class="flex-col flex gap-6 pb-4">
					{#each settings[category] as setting}
						<!-- slack connect is handled with the alert channels settings, smtp_connect is handled in InstanceSetting -->
						{#if setting.fieldType != 'slack_connect'}
							<InstanceSetting
								{openSmtpSettings}
								on:closeDrawer={() => closeDrawer?.()}
								{loading}
								{setting}
								{values}
								{version}
								{oauths}
							/>
						{/if}
					{/each}
				</div>
			</TabContent>
		{/each}
	{/snippet}
</div>

{#if !hideSave}
	<Button on:click={saveSettings} variant="accent">Save settings</Button>
	<div class="pb-8"></div>
{/if}
