import type { ButtonType } from './common/button/model'
import { z } from 'zod'

// Languages that support HTTP request tracing via OTEL proxy
export const OTEL_TRACING_PROXY_LANGUAGES = [
	'nativets',
	'python3',
	'deno',
	'bun',
	'go',
	'bash',
	'rust',
	'csharp',
	'nu',
	'ruby'
] as const

export interface Setting {
	label: string
	description?: string
	placeholder?: string
	cloudonly?: boolean
	ee_only?: string
	tooltip?: string
	key: string
	// If value is not specified for first element, it will automatcally use undefined
	select_items?: {
		label: string
		tooltip?: string
		// If not specified, label will be used
		value?: string
	}[]
	fieldType:
		| 'text'
		| 'number'
		| 'boolean'
		| 'password'
		| 'select'
		| 'select_python'
		| 'textarea'
		| 'codearea'
		| 'seconds'
		| 'email'
		| 'license_key'
		| 'object_store_config'
		| 'critical_error_channels'
		| 'critical_alerts_on_db_oversize'
		| 'slack_connect'
		| 'smtp_connect'
		| 'indexer_rates'
		| 'otel'
		| 'otel_tracing_proxy'
		| 'secret_backend'
	storage: SettingStorage
	advancedToggle?: {
		label: string
		onChange: (values: Record<string, any>) => Record<string, any>
		checked: (values: Record<string, any>) => boolean
	}
	hiddenIfNull?: boolean
	hiddenIfEmpty?: boolean
	hiddenInEe?: boolean
	hideInQuickSetup?: boolean
	requiresReloadOnChange?: boolean
	isValid?: (value: any) => boolean
	validate?: (value: any) => Record<string, string>
	error?: string
	defaultValue?: () => any
	codeAreaLang?: string
	actionButton?: {
		label: string
		onclick: (values: Record<string, any>) => Promise<void>
		variant?: ButtonType.Variant
	}
}

export type SettingStorage = 'setting'

const positiveNumber = z.number().positive('Must be a positive number')

const indexerSettingsSchema = z
	.object({
		writer_memory_budget: positiveNumber.optional(),
		commit_job_max_batch_size: positiveNumber.optional(),
		refresh_index_period: positiveNumber.optional(),
		max_indexed_job_log_size: positiveNumber.optional(),
		commit_log_max_batch_size: positiveNumber.optional(),
		refresh_log_index_period: positiveNumber.optional()
	})
	.passthrough()

function validateIndexerSettings(v: any): Record<string, string> {
	if (!v) return {}
	const result = indexerSettingsSchema.safeParse(v)
	if (result.success) return {}
	const errors: Record<string, string> = {}
	for (const issue of result.error.issues) {
		const field = issue.path[0]?.toString()
		if (field) errors[field] = issue.message
	}
	return errors
}

export const scimSamlSetting: Setting[] = [
	{
		label: 'SCIM token',
		description: 'Token used to authenticate requests from the IdP',
		key: 'scim_token',
		fieldType: 'password',
		placeholder: 'mytoken',
		storage: 'setting',
		ee_only: ''
	},
	{
		label: 'SAML metadata',
		description: 'XML metadata url OR content for the SAML IdP',
		key: 'saml_metadata',
		fieldType: 'textarea',
		placeholder: 'https://dev-2578259.okta.com/app/exkaell8gidiiUWrg5d7/sso/saml/metadata ',
		storage: 'setting',
		ee_only: ''
	}
]

export const settings: Record<string, Setting[]> = {
	Core: [
		{
			label: 'Base url',
			description:
				'Public base url of the instance. <a href="https://www.windmill.dev/docs/advanced/instance_settings#global-users">Learn more</a>',
			key: 'base_url',
			fieldType: 'text',
			placeholder: 'https://windmill.com',
			storage: 'setting',
			error: 'Base url must start with http:// or https:// and not end with / or a space',
			isValid: (value: string | undefined) =>
				value == undefined ||
				(value?.startsWith('http') &&
					value.includes('://') &&
					!value?.endsWith('/') &&
					!value?.endsWith(' '))
		},
		{
			label: 'Email domain',
			description: 'Domain to display in webhooks for email triggers (should match the MX record)',
			key: 'email_domain',
			fieldType: 'text',
			placeholder: 'mail.windmill.com',
			storage: 'setting',
			error: 'Must be a valid domain',
			isValid: (value: string | undefined) =>
				value == undefined ||
				value === '' ||
				/^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$/.test(
					value
				)
		},
		{
			label: 'Request size limit in MB',
			description: 'Maximum size of HTTP requests in MB.',
			cloudonly: true,
			key: 'request_size_limit_mb',
			fieldType: 'number',
			placeholder: '50',
			storage: 'setting'
		},
		{
			label: 'License key',
			description:
				'License key required to use the EE (switch image for windmill-ee). <a href="https://www.windmill.dev/docs/advanced/instance_settings#license-key">Learn more</a>',
			key: 'license_key',
			fieldType: 'license_key',
			placeholder: 'only for EE',
			storage: 'setting'
		},
		{
			label: 'Non-prod instance',
			description:
				'Whether we should consider the reported usage of this instance as non-prod. <a href="https://www.windmill.dev/docs/advanced/instance_settings#non-prod-instance">Learn more</a>',
			key: 'dev_instance',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: '',
			hideInQuickSetup: true
		},
		{
			label: 'App workspace prefix',
			description:
				'When enabled apps will be accessible at /a/{workspace_id}/{custom_path} instead of /a/{custom_path} allowing you to define same custom path for apps in different workspace without conflict',
			key: 'app_workspaced_route',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: '',
			hideInQuickSetup: true
		}
	],
	Jobs: [
		{
			label: 'Job isolation',
			key: 'job_isolation',
			fieldType: 'select',
			description:
				'Isolation mode for job execution. None: no isolation. Unshare: PID namespace isolation via unshare. Nsjail: full nsjail sandboxing. <a href="https://www.windmill.dev/docs/advanced/security_isolation">Learn more</a>',
			storage: 'setting',
			select_items: [
				{
					label: 'None',
					value: 'none'
				},
				{
					label: 'Unshare',
					value: 'unshare'
				},
				{
					label: 'Nsjail',
					value: 'nsjail_sandboxing'
				}
			]
		},
		{
			label: 'Default timeout',
			key: 'job_default_timeout',
			description:
				'Default timeout for individual jobs. <a href="https://www.windmill.dev/docs/core_concepts/jobs#retention-policy">Learn more</a>',
			fieldType: 'seconds',
			storage: 'setting',
			cloudonly: false
		},
		{
			label: 'Max timeout for sync endpoints',
			description:
				'Maximum amount of time (measured in seconds) that a <a href="https://www.windmill.dev/docs/core_concepts/webhooks">sync endpoint</a> is allowed to run before it is forcibly stopped or timed out.',
			key: 'timeout_wait_result',
			cloudonly: true,
			fieldType: 'seconds',
			placeholder: '60',
			storage: 'setting'
		},
		{
			label: 'Keep job directories for debug',
			key: 'keep_job_dir',
			fieldType: 'boolean',
			description: 'Keep Job directories after execution at /tmp/windmill/WORKER/JOB_ID',
			storage: 'setting'
		},
		{
			label: 'Retention period in secs',
			key: 'retention_period_secs',
			description:
				'How long to keep the jobs data in the database (max 30 days on CE). <a href="https://www.windmill.dev/docs/advanced/instance_settings#retention-period-in-secs">Learn more</a>',
			fieldType: 'seconds',
			placeholder: '30',
			storage: 'setting',
			ee_only: 'You can only adjust this setting to above 30 days in the EE version',
			cloudonly: false
		}
	],
	'Object Storage': [
		{
			label: 'Instance object storage',
			description:
				' S3/Azure bucket to store large logs and global cache for Python and Go. <a href="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#instance-object-storage">Learn more</a>',
			key: 'object_store_cache_config',
			fieldType: 'object_store_config',
			storage: 'setting',
			ee_only: '',
			isValid: (v) => {
				if (!v || v.type !== 'Gcs') return true
				return v.serviceAccountKey !== undefined
			}
		},
		{
			label: 'Delete logs from s3 periodically',
			description:
				'Job and service logs are periodically deleted from disk. When this setting is on, they will also be deleted from the object storage.',
			key: 'monitor_logs_on_s3',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		}
	],
	'Private Hub': [
		{
			label: 'Private Hub base url',
			description:
				'Base URL of your Private Hub instance, without trailing slash. <a href="https://www.windmill.dev/docs/core_concepts/private_hub">Learn more</a>',
			placeholder: 'https://hub.company.com',
			key: 'hub_base_url',
			fieldType: 'text',
			storage: 'setting',
			ee_only: '',
			advancedToggle: {
				label: 'I have a different URL for Hub access from end-user browsers',
				onChange(values) {
					if (values['hub_accessible_url']) {
						values['hub_accessible_url'] = null
					} else {
						values['hub_accessible_url'] = values['hub_base_url'] || 'https://hub.company.com'
					}
					return values
				},
				checked: (values) => values['hub_accessible_url'] != null
			},
			requiresReloadOnChange: true
		},
		{
			label: 'Private Hub accessible url',
			description:
				'Base URL accessible from end-user browsers, without trailing slash. <a href="https://www.windmill.dev/docs/core_concepts/private_hub">Learn more</a>',
			key: 'hub_accessible_url',
			fieldType: 'text',
			hiddenIfNull: true,
			storage: 'setting',
			ee_only: '',
			requiresReloadOnChange: true
		},
		{
			label: 'Private Hub API secret',
			description:
				'If access to your Private Hub is restricted, you can set the hub API secret here. <a href="https://www.windmill.dev/docs/core_concepts/private_hub">Learn more</a>',
			key: 'hub_api_secret',
			fieldType: 'password',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Azure OpenAI base path',
			description:
				'All workspaces using an OpenAI resource for Windmill AI will run on the specified deployed model. Format: https://{your-resource-name}.openai.azure.com/openai/deployments/{deployment-id}. <a href="https://www.windmill.dev/docs/core_concepts/ai_generation#azure-openai-advanced-models">Learn more</a>',
			key: 'openai_azure_base_path',
			fieldType: 'text',
			storage: 'setting',
			ee_only: '',
			hiddenIfEmpty: true
		}
	],
	SMTP: [
		{
			label: 'SMTP configuration',
			key: 'smtp_settings',
			fieldType: 'smtp_connect',
			storage: 'setting',
			ee_only: ''
		}
	],
	'Auth/OAuth/SAML': [],
	Registries: [
		{
			label: 'Instance Python Version',
			description: 'Default python version for newly deployed scripts',
			key: 'instance_python_version',
			fieldType: 'select_python',
			// To change latest stable version:
			// 1. Change placeholder in instanceSettings.ts
			// 2. Change LATEST_STABLE_PY in dockerfile
			// 3. Change #[default] annotation for PyVersion in backend
			placeholder: '3.10,3.11,3.12,3.13',
			select_items: [
				{
					label: 'Latest Stable',
					value: 'default',
					tooltip: 'python-3.12'
				},
				{
					label: '3.10'
				},
				{
					label: '3.11'
				},
				{
					label: '3.12'
				},
				{
					label: '3.13'
				}
			],
			storage: 'setting'
		},
		{
			label: 'UV index url',
			description: 'Add private Pip registry',
			key: 'pip_index_url',
			fieldType: 'password',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'UV extra index url',
			description: 'Add private extra Pip registry',
			key: 'pip_extra_index_url',
			fieldType: 'password',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'UV index strategy',
			description:
				'Strategy for resolving packages from multiple indexes. See <a href="https://docs.astral.sh/uv/pip/compatibility/#packages-that-exist-on-multiple-indexes">uv docs</a>',
			key: 'uv_index_strategy',
			fieldType: 'select',
			placeholder: 'unsafe-best-match',
			defaultValue: () => 'unsafe-best-match',
			select_items: [
				{
					label: 'first-index',
					tooltip: 'Only use the first index that contains the package'
				},
				{
					label: 'unsafe-first-match',
					tooltip: 'Search for packages across all indexes, preferring the first match'
				},
				{
					label: 'unsafe-best-match (default)',
					value: 'unsafe-best-match',
					tooltip: 'Search for packages across all indexes, preferring the best match'
				}
			],
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'NPM Registry Configuration (.npmrc)',
			description:
				'Full .npmrc file content for private npm registries. Used by Bun, Deno, and the npm proxy. Takes precedence over the legacy fields below.',
			key: 'npmrc',
			fieldType: 'codearea',
			codeAreaLang: 'ini',
			placeholder:
				'registry=https://registry.mycompany.com/\n//registry.mycompany.com/:_authToken=YOUR_TOKEN\n\n@myorg:registry=https://registry.myorg.com/\n//registry.myorg.com/:_authToken=SCOPED_TOKEN',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Npm config registry (legacy)',
			description: 'Add private npm registry. Prefer using the .npmrc field above.',
			key: 'npm_config_registry',
			fieldType: 'password',
			placeholder: 'https://registry.npmjs.org/:_authToken=npm_FOOBAR',
			storage: 'setting',
			ee_only: '',
			hiddenIfEmpty: true
		},
		{
			label: 'Bunfig install scopes (legacy)',
			description:
				'Add private scoped registries for Bun. Prefer using the .npmrc field above. See: https://bun.sh/docs/install/registries',
			key: 'bunfig_install_scopes',
			fieldType: 'password',
			placeholder: '"@myorg3" = { token = "mytoken", url = "https://registry.myorg.com/" }',
			storage: 'setting',
			ee_only: '',
			hiddenIfEmpty: true
		},
		{
			label: 'Nuget Config',
			description: 'Write a nuget.config file to set custom package sources and credentials',
			key: 'nuget_config',
			fieldType: 'codearea',
			codeAreaLang: 'xml',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Maven/Ivy repositories',
			description: 'Add private Maven/Ivy repositories',
			key: 'maven_repos',
			fieldType: 'password',
			placeholder: 'https://user:password@artifacts.foo.com/maven',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Maven settings.xml',
			description:
				'Write a Maven settings.xml file for custom repositories, mirrors, and credentials',
			key: 'maven_settings_xml',
			fieldType: 'codearea',
			codeAreaLang: 'xml',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Disable default Maven repository',
			description: 'Do not use default Maven repository',
			key: 'no_default_maven',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Ruby Gems repositories',
			description: 'Add private Ruby repositories with credentials. Should end with /',
			key: 'ruby_repos',
			fieldType: 'password',
			placeholder: 'https://user:password@gems.foo.com/',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Cargo registries',
			description: 'Write a .cargo/config.toml to set custom Cargo registries and credentials',
			key: 'cargo_registries',
			fieldType: 'codearea',
			codeAreaLang: 'toml',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'PowerShell Repository URL',
			description: 'Add private PowerShell repository URL',
			key: 'powershell_repo_url',
			placeholder:
				'https://pkgs.dev.azure.com/<org>/<project>/_packaging/<feed>/nuget/v3/index.json',
			fieldType: 'text',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'PowerShell Repository PAT',
			description:
				'Add private PowerShell repository Personal Access Token (optional, for authenticated repositories)',
			key: 'powershell_repo_pat',
			fieldType: 'password',
			storage: 'setting',
			ee_only: ''
		}
	],
	Alerts: [
		{
			label: 'Critical alert channels',
			description:
				'Channels to send critical alerts to. <a href="https://www.windmill.dev/docs/core_concepts/critical_alerts">Learn more</a>',
			key: 'critical_error_channels',
			fieldType: 'critical_error_channels',
			storage: 'setting',
			ee_only: 'Channels other than tracing are only available in the EE version',
			actionButton: {
				label: 'Test all channels',
				onclick: async (values) => {
					const { SettingService } = await import('$lib/gen')
					const { sendUserToast } = await import('$lib/toast')
					try {
						await SettingService.testCriticalChannels({
							requestBody: values.critical_error_channels
						})
						sendUserToast('Test message sent successfully to critical channels', false)
					} catch (error: any) {
						sendUserToast('Failed to send test message: ' + error.message, true)
					}
				},
				variant: 'accent'
			}
		},
		{
			label: 'Mute critical alerts in UI',
			description: 'Enable to mute critical alerts in the UI',
			key: 'critical_alert_mute_ui',
			fieldType: 'boolean',
			storage: 'setting',
			requiresReloadOnChange: true,
			ee_only: 'Critical alerts in UI are only available in the EE version'
		},
		{
			label: 'Slack',
			key: 'slack',
			fieldType: 'slack_connect',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Alert on DB oversize',
			key: 'critical_alerts_on_db_oversize',
			description: 'Alert if DB grows more than specified size',
			fieldType: 'critical_alerts_on_db_oversize',
			placeholder: '100',
			storage: 'setting',
			ee_only: ''
		}
	],
	'OTEL/Prom': [
		{
			label: 'OpenTelemetry',
			key: 'otel',
			fieldType: 'otel',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'HTTP Request Tracing',
			description:
				'Capture HTTP/HTTPS requests from job scripts as OpenTelemetry spans. Visible in job details and exported to your OTEL collector if configured. Toggling restarts workers.',
			key: 'otel_tracing_proxy',
			fieldType: 'otel_tracing_proxy',
			storage: 'setting',
			ee_only: 'HTTP Request Tracing is an EE feature',
			defaultValue: () => ({ enabled: false, enabled_languages: [...OTEL_TRACING_PROXY_LANGUAGES] })
		},
		{
			label: 'Prometheus',
			description:
				'Expose Prometheus metrics for workers and servers on port 8001 at /metrics. <a target="_blank" href="https://www.windmill.dev/docs/advanced/instance_settings#expose-metrics">Learn more</a>',
			key: 'expose_metrics',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		}
	],
	Indexer: [
		{
			label: '',
			key: 'indexer_settings',
			fieldType: 'indexer_rates',
			storage: 'setting',
			validate: validateIndexerSettings
		}
	],

	Telemetry: [
		{
			label: 'Disable telemetry',
			key: 'disable_stats',
			fieldType: 'boolean',
			storage: 'setting',
			hiddenInEe: true
		}
	],
	'Secret Storage': [
		{
			label: 'Backend type',
			description:
				'By default, secrets are encrypted and stored in the database. Enterprise Edition supports HashiCorp Vault as an external secret store.',
			key: 'secret_backend',
			fieldType: 'secret_backend',
			storage: 'setting',
			ee_only: 'HashiCorp Vault integration is an Enterprise Edition feature'
		}
	]
}

export const settingsKeys = Object.keys(settings)

// --- Sidebar navigation for instance settings ---
export const instanceSettingsNavigationGroups = [
	{
		title: 'Core',
		items: [
			{
				id: 'users',
				label: 'Users',
				aiId: 'instance-settings-users',
				aiDescription: 'Instance users settings'
			},
			{
				id: 'general',
				label: 'General',
				aiId: 'instance-settings-general',
				aiDescription: 'Instance general settings'
			},
			{
				id: 'jobs',
				label: 'Jobs',
				aiId: 'instance-settings-jobs',
				aiDescription: 'Instance jobs settings'
			}
		]
	},
	{
		title: 'Authentication',
		items: [
			{
				id: 'sso',
				label: 'SSO',
				aiId: 'instance-settings-sso',
				aiDescription: 'Instance SSO settings'
			},
			{
				id: 'oauth',
				label: 'OAuth',
				aiId: 'instance-settings-oauth',
				aiDescription: 'Instance OAuth settings'
			},
			{
				id: 'scim_saml',
				label: 'SCIM/SAML',
				aiId: 'instance-settings-scim-saml',
				aiDescription: 'Instance SCIM/SAML settings',
				isEE: true
			}
		]
	},
	{
		title: 'Infrastructure',
		items: [
			{
				id: 'smtp',
				label: 'SMTP',
				aiId: 'instance-settings-smtp',
				aiDescription: 'Instance SMTP settings'
			},
			{
				id: 'registries',
				label: 'Registries',
				aiId: 'instance-settings-registries',
				aiDescription: 'Instance registries settings'
			},
			{
				id: 'object_storage',
				label: 'Object Storage',
				aiId: 'instance-settings-object-storage',
				aiDescription: 'Instance object storage settings',
				isEE: true
			}
		]
	},
	{
		title: 'Monitoring',
		items: [
			{
				id: 'alerts',
				label: 'Alerts',
				aiId: 'instance-settings-alerts',
				aiDescription: 'Instance alerts settings',
				isEE: true
			},
			{
				id: 'otel_prom',
				label: 'OTEL/Prometheus',
				aiId: 'instance-settings-otel-prom',
				aiDescription: 'Instance OTEL/Prometheus settings',
				isEE: true
			},
			{
				id: 'indexer',
				label: 'Indexer',
				aiId: 'instance-settings-indexer',
				aiDescription: 'Instance indexer settings',
				isEE: true
			}
		]
	},
	{
		title: 'Advanced',
		items: [
			{
				id: 'private_hub',
				label: 'Private Hub',
				aiId: 'instance-settings-private-hub',
				aiDescription: 'Instance private hub settings',
				isEE: true
			},
			{
				id: 'telemetry',
				label: 'Telemetry',
				aiId: 'instance-settings-telemetry',
				aiDescription: 'Instance telemetry settings'
			},
			{
				id: 'secret_storage',
				label: 'Secret Storage',
				aiId: 'instance-settings-secret-storage',
				aiDescription: 'Instance secret storage settings'
			}
		]
	}
]

export const tabToCategoryMap: Record<string, string> = {
	general: 'Core',
	sso: 'Auth/OAuth/SAML',
	oauth: 'Auth/OAuth/SAML',
	scim_saml: 'Auth/OAuth/SAML',
	smtp: 'SMTP',
	registries: 'Registries',
	alerts: 'Alerts',
	otel_prom: 'OTEL/Prom',
	indexer: 'Indexer',
	telemetry: 'Telemetry',
	secret_storage: 'Secret Storage',
	object_storage: 'Object Storage',
	jobs: 'Jobs',
	private_hub: 'Private Hub'
}

export const tabToAuthSubTab: Record<string, 'sso' | 'oauth' | 'scim'> = {
	sso: 'sso',
	oauth: 'oauth',
	scim_saml: 'scim'
}

// Navigation groups for the initial setup flow (no Users tab)
export const setupNavigationGroups = instanceSettingsNavigationGroups
	.map((group) => ({
		...group,
		items: group.items.filter((item) => item.id !== 'users')
	}))
	.filter((group) => group.items.length > 0)

export const categoryToTabMap: Record<string, string> = {
	Core: 'general',
	SMTP: 'smtp',
	'Auth/OAuth/SAML': 'sso',
	Registries: 'registries',
	Alerts: 'alerts',
	'OTEL/Prom': 'otel_prom',
	Indexer: 'indexer',
	Telemetry: 'telemetry',
	'Secret Storage': 'secret_storage',
	'Object Storage': 'object_storage',
	Jobs: 'jobs',
	'Private Hub': 'private_hub'
}

export interface SearchableSettingItem {
	label: string
	tabId: string
	settingKey?: string
	category: string
	/** Full description text (HTML stripped), used for search matching only — not displayed */
	description?: string
}

/**
 * Extract the label portion from a uFuzzy marked/highlighted string.
 * Only allows `<mark>` and `</mark>` tags through (sanitizes everything else).
 */
export function extractMarkedLabel(marked: string | undefined, labelLength: number): string {
	if (!marked) return ''
	let plainIdx = 0
	let markedIdx = 0
	while (plainIdx < labelLength && markedIdx < marked.length) {
		if (marked[markedIdx] === '<') {
			while (markedIdx < marked.length && marked[markedIdx] !== '>') markedIdx++
			markedIdx++
		} else {
			plainIdx++
			markedIdx++
		}
	}
	// Include any closing </mark> right after
	if (marked.startsWith('</mark>', markedIdx)) {
		markedIdx += '</mark>'.length
	}
	// Sanitize: only allow <mark> and </mark> tags from uFuzzy highlight
	return marked.slice(0, markedIdx).replace(/<(?!\/?mark>)[^>]*>/g, '')
}

export function buildSearchableSettingItems(
	navigationGroups: typeof instanceSettingsNavigationGroups = instanceSettingsNavigationGroups
): SearchableSettingItem[] {
	const items: SearchableSettingItem[] = []

	// Add sidebar navigation items (tab-level)
	for (const group of navigationGroups) {
		for (const navItem of group.items) {
			items.push({
				label: navItem.label,
				tabId: navItem.id,
				category: group.title
			})
		}
	}

	// Add individual settings from each category
	for (const [category, categorySettings] of Object.entries(settings)) {
		const tabId = categoryToTabMap[category]
		if (!tabId) continue
		for (const setting of categorySettings) {
			if (!setting.label) continue
			items.push({
				label: setting.label,
				tabId,
				settingKey: setting.key,
				category,
				description: setting.description?.replace(/<[^>]*>/g, '') ?? ''
			})
		}
	}

	// Add SCIM/SAML settings
	for (const setting of scimSamlSetting) {
		if (!setting.label) continue
		items.push({
			label: setting.label,
			tabId: 'scim_saml',
			settingKey: setting.key,
			category: 'SCIM/SAML',
			description: setting.description?.replace(/<[^>]*>/g, '') ?? ''
		})
	}

	return items
}
