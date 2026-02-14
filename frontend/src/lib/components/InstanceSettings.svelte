<script lang="ts">
	import { scimSamlSetting, settings, settingsKeys } from './instanceSettings'
	import { Alert, Button, Tab, TabContent, Tabs } from '$lib/components/common'
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
	import { Loader2 } from 'lucide-svelte'
	import YAML from 'yaml'
	import Toggle from './Toggle.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'
	import SettingsFooter from './workspaceSettings/SettingsFooter.svelte'
	import SettingsPageHeader from './settings/SettingsPageHeader.svelte'

	interface Props {
		tab?: string
		hideTabs?: boolean
		closeDrawer?: (() => void) | undefined
		authSubTab?: 'sso' | 'oauth' | 'scim'
		onNavigateToTab?: (category: string) => void
		quickSetup?: boolean
		yamlMode?: boolean
	}

	let {
		tab = $bindable('Core'),
		hideTabs = false,
		closeDrawer = () => {},
		authSubTab = $bindable('sso'),
		onNavigateToTab,
		quickSetup = false,
		yamlMode = $bindable(false)
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
		if (!nvalues['base_url']) {
			nvalues['base_url'] = window.location.origin
		}
		if (nvalues['retention_period_secs'] == undefined) {
			nvalues['retention_period_secs'] = 60 * 60 * 24 * 30
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
		if (yamlMode) {
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
				if (x.storage == 'setting' && !deepEqual(initialValues?.[x.key], $values?.[x.key])) {
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

	let downloadingStats = $state(false)
	async function downloadStats() {
		try {
			downloadingStats = true
			const encryptedData = await SettingService.getStats()
			const blob = new Blob([encryptedData], { type: 'application/octet-stream' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `windmill-telemetry-${new Date().toISOString().split('T')[0]}.enc`
			document.body.appendChild(a)
			a.click()
			document.body.removeChild(a)
			URL.revokeObjectURL(url)
			sendUserToast('Telemetry data downloaded')
		} catch (err) {
			throw err
		} finally {
			downloadingStats = false
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
		if (onNavigateToTab) {
			onNavigateToTab('SMTP')
		} else {
			tab = 'SMTP'
		}
	}

	// --- Per-category dirty state tracking ---

	// Trigger to force re-derivation when initialValues changes (after save/load)
	let dirtyCheckTrigger = $state(0)

	function stripEmpty(obj: Record<string, any>): Record<string, any> {
		return Object.fromEntries(
			Object.entries(obj)
				.filter(([_, v]) => v !== undefined && v !== '')
				.map(([k, v]) =>
					v != null && typeof v === 'object' && !Array.isArray(v) ? [k, stripEmpty(v)] : [k, v]
				)
		)
	}

	function getSettingsForCategory(category: string) {
		if (category === 'Auth/OAuth/SAML') {
			return scimSamlSetting
		}
		const base = settings[category] ?? []
		// In quick setup, reorder Core: base settings (without license_key), then extras from Jobs
		if (quickSetup && category === 'Core') {
			const licenseKey = base.find((s) => s.key === 'license_key')
			const baseWithout = base.filter((s) => s.key !== 'license_key')
			const jobSettings = settings['Jobs'] ?? []
			const jobIsolation = jobSettings.find((s) => s.key === 'job_isolation')
			const retentionPeriod = jobSettings.find((s) => s.key === 'retention_period_secs')
			const objectStorage = settings['Object Storage']?.find(
				(s) => s.key === 'object_store_cache_config'
			)
			return [
				...baseWithout,
				...(jobIsolation ? [jobIsolation] : []),
				...(licenseKey ? [licenseKey] : []),
				...(retentionPeriod ? [retentionPeriod] : []),
				...(objectStorage ? [objectStorage] : [])
			]
		}
		return base
	}

	function normalizeValue(value: any): any {
		if (value === undefined || value === null) {
			return undefined
		}
		if (Array.isArray(value) && value.length === 0) {
			return undefined
		}
		if (typeof value === 'object' && Object.keys(value).length === 0) {
			return undefined
		}
		if (typeof value === 'string' && value.trim() === '') {
			return undefined
		}
		if (typeof value == 'boolean' && value === false) {
			return undefined
		}

		return value
	}
	function isDirtyValue(key: string, initialValues: any, currentValues: any): boolean {
		return !deepEqual(normalizeValue(initialValues[key]), normalizeValue(currentValues[key]))
	}

	let dirtyCategories: Record<string, boolean> = $derived.by(() => {
		void dirtyCheckTrigger
		const currentValues = $values
		const result: Record<string, boolean> = {}
		for (const category of settingsKeys) {
			if (category === 'Auth/OAuth/SAML') {
				const scimDirty = scimSamlSetting.some((s) =>
					isDirtyValue(s.key, initialValues, currentValues)
				)
				const oauthsDirty = !deepEqual(stripEmpty(initialOauths), stripEmpty(oauths))
				const requirePreexistingDirty =
					initialRequirePreexistingUserForOauth !== requirePreexistingUserForOauth
				result[category] = scimDirty || oauthsDirty || requirePreexistingDirty
			} else {
				const categorySettings = getSettingsForCategory(category)
				let isDirty = categorySettings.some((s) =>
					isDirtyValue(s.key, initialValues, currentValues)
				)
				let dirtyKey = categorySettings
					.map((s) => s.key)
					.find((key) => isDirtyValue(key, initialValues, currentValues))
				if (dirtyKey) {
					console.log('isDirty', dirtyKey, initialValues[dirtyKey], currentValues[dirtyKey])
				}
				result[category] = isDirty
			}
		}
		return result
	})

	let invalidCategories: Record<string, boolean> = $derived.by(() => {
		const currentValues = $values
		const result: Record<string, boolean> = {}
		for (const category of settingsKeys) {
			const categorySettings = getSettingsForCategory(category)
			result[category] = categorySettings.some((s) => {
				if (s.isValid && !s.isValid(currentValues?.[s.key])) return true
				if (s.validate) {
					const errors = s.validate(currentValues?.[s.key])
					return Object.keys(errors).length > 0
				}
				return false
			})
		}
		return result
	})

	export function isDirty(category: string): boolean {
		return dirtyCategories[category] ?? false
	}

	export function discardCategory(category: string) {
		if (category === 'Auth/OAuth/SAML') {
			for (const s of scimSamlSetting) {
				const v = initialValues[s.key]
				$values[s.key] = v !== undefined ? JSON.parse(JSON.stringify(v)) : undefined
			}
			oauths = JSON.parse(JSON.stringify(initialOauths))
			requirePreexistingUserForOauth = initialRequirePreexistingUserForOauth
			const account_identifier =
				initialOauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier
			snowflakeAccountIdentifier = account_identifier ?? ''
		} else {
			const categorySettings = getSettingsForCategory(category)
			for (const s of categorySettings) {
				const v = initialValues[s.key]
				$values[s.key] = v !== undefined ? JSON.parse(JSON.stringify(v)) : undefined
			}
		}
	}

	export async function saveCategorySettings(category: string) {
		// Category-specific pre-processing
		if (category === 'Auth/OAuth/SAML') {
			if (
				oauths?.snowflake_oauth &&
				oauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier !==
					snowflakeAccountIdentifier
			) {
				setupSnowflakeUrls()
			}
		}

		if (category === 'Alerts' && $values?.critical_error_channels) {
			$values.critical_error_channels = $values.critical_error_channels.filter((entry: any) => {
				if (!entry || typeof entry !== 'object') return false
				if ('teams_channel' in entry) return isValidTeamsChannel(entry.teams_channel)
				if ('slack_channel' in entry)
					return typeof entry.slack_channel === 'string' && entry.slack_channel.trim() !== ''
				if ('email' in entry) return typeof entry.email === 'string' && entry.email.trim() !== ''
				return false
			})
		}

		if (
			category === 'Core' &&
			$values?.['license_key'] &&
			typeof $values['license_key'] === 'string'
		) {
			$values['license_key'] = $values['license_key'].trim()
		}

		let shouldReloadPage = false
		const categorySettings = getSettingsForCategory(category)

		let licenseKeySet = false
		await Promise.all(
			categorySettings
				.filter((x) => {
					return (
						x.storage === 'setting' &&
						!deepEqual(initialValues?.[x.key], $values?.[x.key]) &&
						($values?.[x.key] !== '' ||
							initialValues?.[x.key] !== undefined ||
							initialValues?.[x.key] !== null)
					)
				})
				.map(async (x) => {
					if (x.key === 'license_key') licenseKeySet = true
					if (x.requiresReloadOnChange) shouldReloadPage = true
					let value = $values?.[x.key]
					if (x.fieldType === 'codearea' && typeof value === 'string' && value.trim() === '') {
						value = undefined
					}
					return await SettingService.setGlobal({
						key: x.key,
						requestBody: { value }
					})
				})
		)

		// Update only the saved category's initial values
		for (const s of categorySettings) {
			const v = $values[s.key]
			initialValues[s.key] = v !== undefined ? JSON.parse(JSON.stringify(v)) : undefined
		}

		// Handle Auth/OAuth/SAML-specific saves
		if (category === 'Auth/OAuth/SAML') {
			if (!deepEqual(stripEmpty(initialOauths), stripEmpty(oauths))) {
				await SettingService.setGlobal({
					key: 'oauths',
					requestBody: { value: oauths }
				})
				initialOauths = JSON.parse(JSON.stringify(oauths))
			}
			if (initialRequirePreexistingUserForOauth !== requirePreexistingUserForOauth) {
				await SettingService.setGlobal({
					key: 'require_preexisting_user_for_oauth',
					requestBody: { value: requirePreexistingUserForOauth }
				})
				initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
			}
		}

		if (licenseKeySet) setLicense()

		// Force dirty state re-check
		dirtyCheckTrigger++

		if (shouldReloadPage) {
			sendUserToast('Settings updated, reloading page...')
			await sleep(1000)
			window.location.reload()
		} else {
			sendUserToast('Settings updated')
			dispatch('saved')
		}
	}

	let yamlCode = $state('')
	let yamlEditor: SimpleEditor | undefined = $state(undefined)
	let yamlError = $state('')
	let showSensitive = $state(false)

	const SENSITIVE_UNCHANGED = '__SENSITIVE_AND_UNCHANGED__'

	const sensitiveKeys: Set<string> = new Set(
		[...Object.values(settings), scimSamlSetting]
			.flatMap((s) => Object.values(s))
			.filter((s) => s.fieldType === 'password' || s.fieldType === 'license_key')
			.map((s) => s.key)
	)

	// Settings that should never appear in YAML export/import
	const excludedKeys: Set<string> = new Set([
		'custom_instance_pg_databases',
		'ducklake_settings',
		'ducklake_user_pg_pwd'
	])

	function maskSensitive(obj: Record<string, any>): Record<string, any> {
		const masked: Record<string, any> = {}
		for (const [key, value] of Object.entries(obj)) {
			if (key === 'oauths' && typeof value === 'object' && value !== null) {
				// Mask the 'secret' field inside each oauth provider
				const maskedOauths: Record<string, any> = {}
				for (const [provider, config] of Object.entries(value as Record<string, any>)) {
					if (typeof config === 'object' && config !== null && 'secret' in config) {
						maskedOauths[provider] = { ...config, secret: SENSITIVE_UNCHANGED }
					} else {
						maskedOauths[provider] = config
					}
				}
				masked[key] = maskedOauths
			} else if (sensitiveKeys.has(key) && value != null && value !== '') {
				masked[key] = SENSITIVE_UNCHANGED
			} else {
				masked[key] = value
			}
		}
		return masked
	}

	function buildYamlObject(): Record<string, any> {
		const obj: Record<string, any> = {}
		for (const [key, value] of Object.entries($values)) {
			if (!excludedKeys.has(key)) {
				obj[key] = value
			}
		}
		if (oauths && Object.keys(oauths).length > 0) {
			obj['oauths'] = oauths
		}
		if (requirePreexistingUserForOauth) {
			obj['require_preexisting_user_for_oauth'] = requirePreexistingUserForOauth
		}
		return showSensitive ? obj : maskSensitive(obj)
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

			// Restore sensitive values that were not changed
			if ('oauths' in parsed && typeof parsed['oauths'] === 'object') {
				for (const [provider, config] of Object.entries(parsed['oauths'] as Record<string, any>)) {
					if (
						typeof config === 'object' &&
						config !== null &&
						config.secret === SENSITIVE_UNCHANGED
					) {
						config.secret = oauths?.[provider]?.secret
					}
				}
				oauths = parsed['oauths'] ?? {}
				delete parsed['oauths']
			}
			if ('require_preexisting_user_for_oauth' in parsed) {
				requirePreexistingUserForOauth = parsed['require_preexisting_user_for_oauth'] ?? false
				delete parsed['require_preexisting_user_for_oauth']
			}

			// Restore unchanged sensitive settings
			for (const key of sensitiveKeys) {
				if (key in parsed && parsed[key] === SENSITIVE_UNCHANGED) {
					parsed[key] = $values[key]
				}
			}

			// Preserve excluded keys from current form state
			for (const key of excludedKeys) {
				if (key in $values) {
					parsed[key] = $values[key]
				}
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

	let prevYamlMode = false
	$effect(() => {
		if (yamlMode && !prevYamlMode) {
			syncFormToYaml()
		} else if (!yamlMode && prevYamlMode) {
			if (!syncYamlToForm()) {
				// Reset toggle back to YAML on parse failure
				yamlMode = true
			}
		}
		prevYamlMode = yamlMode
	})

	function handleShowSensitiveToggle(checked: boolean) {
		// Sync any in-progress edits back to form state before re-rendering
		syncYamlToForm()
		showSensitive = checked
		syncFormToYaml()
	}

	let diffCategory: string | null = $state(null)

	function buildCategoryDiff(category: string): { original: string; modified: string } {
		const categorySettings = getSettingsForCategory(category)
		const original: Record<string, any> = {}
		const modified: Record<string, any> = {}

		for (const s of categorySettings) {
			if (initialValues[s.key] !== undefined) {
				original[s.key] = initialValues[s.key]
			}
			if ($values[s.key] !== undefined) {
				modified[s.key] = $values[s.key]
			}
		}

		if (category === 'Auth/OAuth/SAML') {
			original['oauths'] = initialOauths
			modified['oauths'] = oauths
			original['require_preexisting_user_for_oauth'] = initialRequirePreexistingUserForOauth
			modified['require_preexisting_user_for_oauth'] = requirePreexistingUserForOauth
		}

		return {
			original: YAML.stringify(original),
			modified: YAML.stringify(modified)
		}
	}
</script>

<div class="pb-12">
	{#if yamlMode}
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<div class="flex items-center justify-end gap-4 mb-2">
			<Toggle
				checked={showSensitive}
				on:change={(e) => handleShowSensitiveToggle(e.detail)}
				options={{ right: 'Show sensitive values' }}
				size="xs"
			/>
		</div>
		<div class="border rounded w-full h-[calc(100vh-12rem)]">
			{#await import('$lib/components/SimpleEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					bind:this={yamlEditor}
					class="h-full"
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
		{@render categoryContent(tab)}
	{:else}
		<Tabs bind:selected={tab}>
			{#each settingsKeys as category}
				<Tab value={category} label={category}></Tab>
			{/each}

			{#snippet content()}
				<div class="pt-4"></div>
				{#each Object.keys(settings) as category}
					<TabContent value={category}>
						{@render categoryContent(category)}
					</TabContent>
				{/each}
			{/snippet}
		</Tabs>
	{/if}

	{#snippet categoryContent(category: string)}
		{#if diffCategory === category}
			<div class="w-full h-[calc(100vh-12rem)]">
				{#await import('$lib/components/DiffEditor.svelte')}
					<Loader2 class="animate-spin m-4" />
				{:then Module}
					{@const diff = buildCategoryDiff(category)}
					<Module.default
						open={true}
						className="!h-full"
						defaultLang="yaml"
						defaultOriginal={diff.original}
						defaultModified={diff.modified}
						readOnly
					/>
				{/await}
			</div>
		{:else if category == 'Core'}
			<SettingsPageHeader
				title="Core"
				description="Configure the core settings of your Windmill instance."
				link="https://www.windmill.dev/docs/advanced/instance_settings"
			/>
		{:else if category == 'SMTP'}
			<SettingsPageHeader
				title="SMTP"
				description="Setting SMTP unlocks sending emails upon adding new users to the workspace or the instance or sending critical alerts via email."
				link="https://www.windmill.dev/docs/advanced/instance_settings#smtp"
			/>
		{:else if category == 'Registries'}
			<SettingsPageHeader
				title="Registries"
				description="Add private registries for Pip, Bun and npm."
				link="https://www.windmill.dev/docs/advanced/imports"
			/>
		{:else if category == 'Alerts'}
			<SettingsPageHeader
				title="Alerts"
				description="Critical alerts automatically notify administrators about system events like job crashes, license issues, worker failures, and queue delays through email, Slack, or Teams."
				link="https://www.windmill.dev/docs/core_concepts/critical_alerts"
			/>
		{:else if category == 'OTEL/Prom'}
			<SettingsPageHeader
				title="OTEL/Prometheus"
				description="Configure OpenTelemetry and Prometheus metrics export for monitoring your Windmill instance."
				link="https://www.windmill.dev/docs/core_concepts/otel"
			/>
		{:else if category == 'Indexer'}
			<SettingsPageHeader
				title="Indexer"
				description="The indexer service unlocks full text search across jobs and service logs. It requires spinning up its own separate container."
				link="https://www.windmill.dev/docs/core_concepts/search_bar#setup"
			/>
			{#if !$enterpriseLicense}
				<Alert
					type="info"
					title="Full text search across jobs and service logs is an EE feature"
					class="mb-2"
				/>
			{/if}
		{:else if category == 'Telemetry'}
			<SettingsPageHeader title="Telemetry" />
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
					On Enterprise Edition, you must send data to check that usage is in line with the terms of
					the subscription. You can either enable telemetry or regularly send usage data by clicking
					the button below. For air-gapped instances, you can download the telemetry data and send
					it manually.
				</div>
				<div class="flex gap-2 mb-4">
					<Button
						on:click={sendStats}
						variant="default"
						btnClasses="w-auto"
						loading={sendingStats}
						size="xs"
					>
						Send usage
					</Button>
					<Button
						on:click={downloadStats}
						variant="default"
						btnClasses="w-auto"
						loading={downloadingStats}
						size="xs"
					>
						Download usage
					</Button>
				</div>
			{/if}
		{:else if category == 'Jobs'}
			<SettingsPageHeader
				title="Jobs"
				description="Configure default timeouts and retention policies for job execution."
				link="https://www.windmill.dev/docs/advanced/instance_settings#jobs"
			/>
		{:else if category == 'Object Storage'}
			<SettingsPageHeader
				title="Object Storage"
				description="Configure S3-compatible storage for large logs and distributed dependency caching."
				link="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill"
			/>
		{:else if category == 'Private Hub'}
			<SettingsPageHeader
				title="Private Hub"
				description="Connect to a Private Hub instance for sharing custom scripts and integrations."
				link="https://www.windmill.dev/docs/core_concepts/private_hub"
			/>
		{:else if category == 'Secret Storage'}
			<SettingsPageHeader
				title="Secret Storage"
				description="Configure where secrets (secret variables) are stored."
				link="https://www.windmill.dev/docs/core_concepts/workspace_secret_encryption"
			/>
		{:else if category == 'Auth/OAuth/SAML'}
			<AuthSettings
				bind:oauths
				bind:snowflakeAccountIdentifier
				bind:requirePreexistingUserForOauth
				baseUrl={$values?.base_url}
				bind:tab={authSubTab}
				{hideTabs}
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

		{#if diffCategory !== category}
		<div class="flex-col flex gap-6 pb-6">
			{#each settings[category] as setting}
				<!-- slack connect is handled with the alert channels settings, smtp_connect is handled in InstanceSetting -->
				{#if setting.fieldType != 'slack_connect' && !(quickSetup && setting.hideInQuickSetup) && !(quickSetup && category === 'Core' && setting.key === 'license_key')}
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
			{#if quickSetup && category === 'Core'}
				{@const licenseKeySetting = settings['Core'].find((s) => s.key === 'license_key')}
				{@const extraSettings = [
					...settings['Jobs'].filter((s) => s.key === 'job_isolation'),
					...(licenseKeySetting ? [licenseKeySetting] : []),
					...settings['Jobs'].filter((s) => s.key === 'retention_period_secs'),
					...(settings['Object Storage']?.filter((s) => s.key === 'object_store_cache_config') ??
						[])
				]}
				{#each extraSettings as setting}
					<InstanceSetting
						{openSmtpSettings}
						on:closeDrawer={() => closeDrawer?.()}
						{loading}
						{setting}
						{values}
						{version}
						{oauths}
					/>
				{/each}
			{/if}
		</div>
		{/if}

		{#if !loading && !quickSetup}
			<SettingsFooter
				hasUnsavedChanges={dirtyCategories[category] ?? false}
				disabled={invalidCategories[category] ?? false}
				onSave={() => saveCategorySettings(category)}
				onDiscard={() => discardCategory(category)}
				onShowDiff={() => {
					diffCategory = diffCategory === category ? null : category
				}}
				diffOpen={diffCategory === category}
				saveLabel={`Save ${category.toLowerCase()} settings`}
				class="bg-surface"
			/>
		{/if}
	{/snippet}
</div>
