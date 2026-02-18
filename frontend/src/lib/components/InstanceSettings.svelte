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
	import { ExternalLink, Loader2 } from 'lucide-svelte'
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
		diffMode?: boolean
		hasUnsavedChanges?: boolean
	}

	let {
		tab = $bindable('Core'),
		hideTabs = false,
		closeDrawer = () => {},
		authSubTab = $bindable('sso'),
		onNavigateToTab,
		quickSetup = false,
		yamlMode = $bindable(false),
		diffMode = $bindable(false),
		hasUnsavedChanges = $bindable(false)
	}: Props = $props()

	let values: Writable<Record<string, any>> = writable({})
	let initialOauths: Record<string, any> = $state({})
	let initialRequirePreexistingUserForOauth: boolean = $state(false)
	let requirePreexistingUserForOauth: boolean = $state(false)

	let initialValues: Record<string, any> = $state({})
	let baseUrlIsFallback = $state(false)
	let snowflakeAccountIdentifier = $state('')
	let version: string = $state('')
	let loading = $state(true)

	export function getVersion(): string {
		return version
	}

	export function getLicenseKey(): string {
		return $values?.['license_key'] ?? ''
	}

	loadSettings()
	loadVersion()

	const dispatch = createEventDispatcher()

	async function loadVersion() {
		version = await SettingsService.backendVersion()
	}
	let oauths: Record<string, any> = $state({})

	/** Ensure object/array-typed settings have a non-null default for the form UI */
	const formDefaults: Record<string, any> = {
		smtp_settings: {},
		otel: {},
		indexer_settings: {},
		critical_error_channels: []
	}

	function applyFormDefaults(vals: Record<string, any>): void {
		for (const [key, defaultVal] of Object.entries(formDefaults)) {
			if (vals[key] == undefined) {
				vals[key] = typeof defaultVal === 'object' ? JSON.parse(JSON.stringify(defaultVal)) : defaultVal
			}
		}
	}

	async function loadSettings() {
		loading = true

		// Bulk-load all settings in a single API call
		const config = await SettingService.getInstanceConfig()
		const gs = (config.global_settings ?? {}) as Record<string, any>

		initialOauths = gs['oauths'] ?? {}
		requirePreexistingUserForOauth = gs['require_preexisting_user_for_oauth'] ?? false
		initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
		oauths = JSON.parse(JSON.stringify(initialOauths))

		let nvalues: Record<string, any> = { ...gs }

		baseUrlIsFallback = !nvalues['base_url']
		if (nvalues['retention_period_secs'] == undefined) {
			nvalues['retention_period_secs'] = 60 * 60 * 24 * 30
		}
		applyFormDefaults(nvalues)

		// Snapshot initialValues before applying the base_url fallback so that
		// the dirty-check detects the unsaved default and enables the Save button.
		initialValues = JSON.parse(JSON.stringify(nvalues))

		if (baseUrlIsFallback) {
			nvalues['base_url'] = window.location.origin
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
			const allSettings = [...Object.values(settings), scimSamlSetting].flat()
			let licenseKeySet = false
			for (const s of allSettings) {
				if (s.storage === 'setting' && !deepEqual(initialValues?.[s.key], $values?.[s.key])) {
					if (s.key === 'license_key') {
						licenseKeySet = true
					}
					if (s.requiresReloadOnChange) {
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

			await SettingService.setInstanceConfig({
				requestBody: { global_settings: globalSettings }
			})

			initialValues = JSON.parse(JSON.stringify($values))
			initialOauths = JSON.parse(JSON.stringify(oauths))
			initialRequirePreexistingUserForOauth = requirePreexistingUserForOauth
			baseUrlIsFallback = false

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

	// --- Dirty state tracking (YAML-based) ---

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

	function normalizeValue(value: any, key?: string): any {
		if (value == null) return undefined
		if (value === false) return undefined
		if (typeof value === 'string' && value.trim() === '') return undefined
		if (Array.isArray(value) && value.length === 0) return undefined
		if (typeof value === 'object' && Object.keys(value).length === 0) return undefined

		// Key-specific defaults: these values are equivalent to "not set"
		if (key === 'secret_backend') {
			if (typeof value === 'object' && value?.type === 'Database' && Object.keys(value).length === 1) {
				return undefined
			}
		}
		if (key === 'automate_username_creation' && value === true) {
			return undefined
		}
		if (key === 'critical_alerts_on_db_oversize' && typeof value === 'object') {
			if (!value.enabled && (!value.value || value.value === 0)) {
				return undefined
			}
		}

		return value
	}
	function buildCategoryYaml(
		category: string,
		vals: Record<string, any>,
		oauthsObj: Record<string, any>,
		reqPreexisting: boolean
	): string {
		const categorySettings = getSettingsForCategory(category)
		const obj: Record<string, any> = {}
		for (const s of categorySettings) {
			const normalized = normalizeValue(vals[s.key], s.key)
			if (normalized !== undefined) {
				obj[s.key] = vals[s.key]
			}
		}
		if (category === 'Auth/OAuth/SAML') {
			if (Object.keys(stripEmpty(oauthsObj)).length > 0) {
				obj['oauths'] = oauthsObj
			}
			if (reqPreexisting) {
				obj['require_preexisting_user_for_oauth'] = reqPreexisting
			}
		}
		return YAML.stringify(obj)
	}

	let dirtyCategories: Record<string, boolean> = $derived.by(() => {
		const result: Record<string, boolean> = {}
		for (const category of settingsKeys) {
			const initialYaml = buildCategoryYaml(
				category,
				initialValues,
				initialOauths,
				initialRequirePreexistingUserForOauth
			)
			const currentYaml = buildCategoryYaml(
				category,
				$values,
				oauths,
				requirePreexistingUserForOauth
			)
			result[category] = initialYaml !== currentYaml
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

	export function discardAll() {
		// Reset all values to initial state (deep copy to avoid reference sharing)
		$values = JSON.parse(JSON.stringify(initialValues))
		oauths = JSON.parse(JSON.stringify(initialOauths))
		requirePreexistingUserForOauth = initialRequirePreexistingUserForOauth
		const account_identifier =
			initialOauths?.snowflake_oauth?.connect_config?.extra_params?.account_identifier
		snowflakeAccountIdentifier = account_identifier ?? ''
		if (yamlMode) {
			syncFormToYaml()
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
		if (categorySettings.some((s) => s.key === 'base_url')) {
			baseUrlIsFallback = false
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
	let yamlCodeInitial = $state('')
	let yamlEditor: SimpleEditor | undefined = $state(undefined)
	let yamlError = $state('')
	let showSensitive = $state(false)

	const SENSITIVE_UNCHANGED = '__SENSITIVE_AND_UNCHANGED__'

	const sensitiveKeys: Set<string> = new Set([
		...[...Object.values(settings), scimSamlSetting]
			.flatMap((s) => Object.values(s))
			.filter((s) => s.fieldType === 'password' || s.fieldType === 'license_key')
			.map((s) => s.key),
		'ducklake_user_pg_pwd',
		'jwt_secret'
	])

	// Settings that should never appear in YAML export/import
	const excludedKeys: Set<string> = new Set([])

	// Nested fields inside object-valued settings that contain secrets.
	// Each entry maps a top-level key to its sensitive sub-field names.
	const nestedSensitiveFields: Record<string, string[]> = {
		smtp_settings: ['smtp_password'],
		secret_backend: ['token'],
		object_store_cache_config: ['secret_key', 'serviceAccountKey'],
		custom_instance_pg_databases: ['user_pwd'],
		rsa_keys: ['private_key']
	}

	/** Returns SENSITIVE_UNCHANGED if the value is non-empty and matches the initial */
	function maskField(current: any, initial: any): string | undefined {
		if (current != null && current !== '' && current === initial) return SENSITIVE_UNCHANGED
		return undefined
	}

	function maskSensitive(obj: Record<string, any>): Record<string, any> {
		const masked: Record<string, any> = {}
		for (const [key, value] of Object.entries(obj)) {
			if (key === 'oauths' && typeof value === 'object' && value !== null) {
				const maskedOauths: Record<string, any> = {}
				for (const [provider, config] of Object.entries(value as Record<string, any>)) {
					if (typeof config === 'object' && config !== null && 'secret' in config) {
						const m = maskField(config.secret, initialOauths?.[provider]?.secret)
						maskedOauths[provider] = m ? { ...config, secret: m } : config
					} else {
						maskedOauths[provider] = config
					}
				}
				masked[key] = maskedOauths
			} else if (key in nestedSensitiveFields && typeof value === 'object' && value !== null) {
				const cp = { ...value }
				const init = initialValues?.[key]
				for (const field of nestedSensitiveFields[key]) {
					const m = maskField(
						field === 'serviceAccountKey' ? JSON.stringify(cp[field]) : cp[field],
						field === 'serviceAccountKey' ? JSON.stringify(init?.[field]) : init?.[field]
					)
					if (m) cp[field] = m
				}
				masked[key] = cp
			} else if (sensitiveKeys.has(key) && value != null && value !== '') {
				masked[key] = value === initialValues?.[key] ? SENSITIVE_UNCHANGED : value
			} else {
				masked[key] = value
			}
		}
		return masked
	}

	/**
	 * Builds a sorted YAML string of all instance settings.
	 * - normalize: strip keys whose values match the default (empty/falsy or key-specific defaults)
	 * - mask: replace sensitive values with placeholder (for display)
	 */
	function buildSettingsYaml(
		vals: Record<string, any>,
		oauthsObj: Record<string, any>,
		reqPreexisting: boolean,
		opts: { normalize?: boolean; mask?: boolean } = {}
	): string {
		// Merge all settings (including oauths) into one object so they sort together
		const merged: Record<string, any> = { ...vals }
		if (oauthsObj && Object.keys(stripEmpty(oauthsObj)).length > 0) {
			merged['oauths'] = oauthsObj
		}
		if (reqPreexisting) {
			merged['require_preexisting_user_for_oauth'] = reqPreexisting
		}
		const obj: Record<string, any> = {}
		for (const key of Object.keys(merged).sort()) {
			if (excludedKeys.has(key)) continue
			if (opts.normalize && normalizeValue(merged[key], key) === undefined) continue
			obj[key] = merged[key]
		}
		// Strip runtime-only `databases` sub-field from custom_instance_pg_databases
		if (obj['custom_instance_pg_databases']?.databases) {
			obj['custom_instance_pg_databases'] = { ...obj['custom_instance_pg_databases'] }
			delete obj['custom_instance_pg_databases'].databases
		}
		return YAML.stringify(opts.mask ? maskSensitive(obj) : obj)
	}

	function syncFormToYaml() {
		yamlCode = buildSettingsYaml($values, oauths, requirePreexistingUserForOauth, {
			normalize: true,
			mask: !showSensitive
		})
		yamlCodeInitial = yamlCode
		yamlEditor?.setCode(yamlCode)
		yamlError = ''
	}

	function syncYamlToForm(): boolean {
		try {
			// Flush the editor's current content (bypasses the 200ms debounce in SimpleEditor)
			const currentCode = yamlEditor?.getCode() ?? yamlCode
			if (currentCode !== yamlCode) {
				yamlCode = currentCode
			}
			const parsed = YAML.parse(yamlCode)
			if (typeof parsed !== 'object' || parsed === null) {
				sendUserToast('YAML must be a mapping (key: value)', true)
				return false
			}

			// Restore sensitive values that were not changed (placeholder → original value)
			if ('oauths' in parsed && typeof parsed['oauths'] === 'object') {
				for (const [provider, config] of Object.entries(parsed['oauths'] as Record<string, any>)) {
					if (
						typeof config === 'object' &&
						config !== null &&
						config.secret === SENSITIVE_UNCHANGED
					) {
						config.secret = initialOauths?.[provider]?.secret
					}
				}
				oauths = parsed['oauths'] ?? {}
				delete parsed['oauths']
			}
			if ('require_preexisting_user_for_oauth' in parsed) {
				requirePreexistingUserForOauth = parsed['require_preexisting_user_for_oauth'] ?? false
				delete parsed['require_preexisting_user_for_oauth']
			}

			// Restore unchanged sensitive settings (placeholder → original value)
			for (const key of sensitiveKeys) {
				if (key in parsed && parsed[key] === SENSITIVE_UNCHANGED) {
					parsed[key] = initialValues?.[key]
				}
			}
			// Restore nested sensitive fields
			for (const [parentKey, fields] of Object.entries(nestedSensitiveFields)) {
				if (parsed[parentKey] && typeof parsed[parentKey] === 'object') {
					const init = initialValues?.[parentKey]
					for (const field of fields) {
						if (parsed[parentKey][field] === SENSITIVE_UNCHANGED) {
							parsed[parentKey][field] = init?.[field]
						}
					}
				}
			}

			// Preserve excluded keys from current form state
			for (const key of excludedKeys) {
				if (key in $values) {
					parsed[key] = $values[key]
				}
			}

			// Preserve runtime-only `databases` sub-field in custom_instance_pg_databases
			const existingDatabases = initialValues?.['custom_instance_pg_databases']?.databases
			if (existingDatabases && parsed['custom_instance_pg_databases']) {
				parsed['custom_instance_pg_databases'].databases = existingDatabases
			}

			$values = parsed
			applyFormDefaults($values)

			yamlError = ''
			return true
		} catch (e) {
			yamlError = String(e)
			sendUserToast('Invalid YAML: ' + e, true)
			return false
		}
	}

	let prevYamlMode = false
	let prevLoading = true
	$effect(() => {
		if (yamlMode && !prevYamlMode) {
			syncFormToYaml()
		} else if (!yamlMode && prevYamlMode) {
			if (!syncYamlToForm()) {
				// Reset toggle back to YAML on parse failure
				yamlMode = true
			}
		} else if (yamlMode && prevLoading && !loading) {
			// Settings just finished loading while in YAML mode
			syncFormToYaml()
		}
		prevYamlMode = yamlMode
		prevLoading = loading
	})

	function handleShowSensitiveToggle(checked: boolean) {
		// Sync any in-progress edits back to form state before re-rendering
		syncYamlToForm()
		showSensitive = checked
		syncFormToYaml()
	}

	/** Call before entering diff mode to sync YAML edits into form state */
	export function syncBeforeDiff(): boolean {
		if (yamlMode) {
			return syncYamlToForm()
		}
		return true
	}

	export function buildFullDiff(): { original: string; modified: string } {
		return {
			original: buildSettingsYaml(
				initialValues,
				initialOauths,
				initialRequirePreexistingUserForOauth,
				{ normalize: true }
			),
			modified: buildSettingsYaml($values, oauths, requirePreexistingUserForOauth, {
				normalize: true
			})
		}
	}

	$effect(() => {
		if (yamlMode) {
			// In YAML mode, compare editor content against snapshot taken on entry
			hasUnsavedChanges = yamlCodeInitial !== '' && yamlCode !== yamlCodeInitial
		} else {
			// Reuse per-category dirty tracking instead of rebuilding full YAML
			hasUnsavedChanges = Object.values(dirtyCategories).some(Boolean)
		}
	})
</script>

<div class="pb-12">
	{#if diffMode}
		<div class="w-full h-[calc(100vh-8rem)]">
			{#await import('$lib/components/DiffEditor.svelte')}
				<Loader2 class="animate-spin m-4" />
			{:then Module}
				{@const diff = buildFullDiff()}
				<Module.default
					open={true}
					className="!h-full"
					defaultLang="yaml"
					defaultOriginal={diff.original}
					defaultModified={diff.modified}
					readOnly
					inlineDiff={true}
				/>
			{/await}
		</div>
	{:else if yamlMode}
		<p class="text-2xs text-tertiary mb-2">
			Use this YAML to manage instance settings as code.
			<a
				href="https://www.windmill.dev/docs/advanced/instance_settings#kubernetes-operator"
				target="_blank"
				rel="noopener noreferrer"
			>Learn more <ExternalLink size={12} class="inline-block" /></a>
		</p>
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
		{#if category == 'Core'}
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
			{#if !$enterpriseLicense}
				<Alert
					type="info"
					title="Private registries configuration is an EE feature"
					class="mb-2"
				/>
			{/if}
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
						warning={setting.key === 'base_url' && baseUrlIsFallback ? 'Auto-detected from browser — not yet saved' : undefined}
					/>
				{/if}
				{#if quickSetup && category === 'Core' && setting.key === 'base_url'}
					{@const licenseKeySetting = settings['Core'].find((s) => s.key === 'license_key')}
					{#if licenseKeySetting}
						<InstanceSetting
							{openSmtpSettings}
							on:closeDrawer={() => closeDrawer?.()}
							{loading}
							setting={licenseKeySetting}
							{values}
							{version}
							{oauths}
						/>
					{/if}
				{/if}
			{/each}
			{#if quickSetup && category === 'Core'}
				{@const extraSettings = [
					...settings['Jobs'].filter((s) => s.key === 'job_isolation'),
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

		{#if !loading && !quickSetup && !hideTabs}
			<SettingsFooter
				hasUnsavedChanges={dirtyCategories[category] ?? false}
				disabled={invalidCategories[category] ?? false}
				onSave={() => saveCategorySettings(category)}
				onDiscard={() => discardCategory(category)}
				saveLabel={`Save ${category.toLowerCase()} settings`}
				class="bg-surface"
			/>
		{/if}
	{/snippet}
</div>
