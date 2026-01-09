import type { ButtonType } from './common/button/model'

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
	storage: SettingStorage
	advancedToggle?: {
		label: string
		onChange: (values: Record<string, any>) => Record<string, any>
		checked: (values: Record<string, any>) => boolean
	}
	hiddenIfNull?: boolean
	hiddenIfEmpty?: boolean
	requiresReloadOnChange?: boolean
	isValid?: (value: any) => boolean
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

export const scimSamlSetting: Setting[] = [
	{
		label: 'SCIM token',
		description: 'Token used to authenticate requests from the IdP',
		key: 'scim_token',
		fieldType: 'text',
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
			description:
				'Domain to display in webhooks for <a href="https://www.windmill.dev/docs/advanced/email_triggers">email triggers</a> (should match the MX record)',
			key: 'email_domain',
			fieldType: 'text',
			storage: 'setting',
			placeholder: 'mail.windmill.com'
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
			label: 'Default timeout',
			key: 'job_default_timeout',
			description:
				'Default timeout for individual jobs. <a href="https://www.windmill.dev/docs/core_concepts/jobs#retention-policy">Learn more</a>',
			fieldType: 'seconds',
			storage: 'setting',
			cloudonly: false
		},
		{
			label: 'Keep job directories for debug',
			key: 'keep_job_dir',
			fieldType: 'boolean',
			description: 'Keep Job directories after execution at /tmp/windmill/WORKER/JOB_ID',
			storage: 'setting'
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
			label: 'License key',
			description:
				'License key required to use the EE (switch image for windmill-ee). <a href="https://www.windmill.dev/docs/advanced/instance_settings#license-key">Learn more</a>',
			key: 'license_key',
			fieldType: 'license_key',
			placeholder: 'only needed to prepare upgrade to EE',
			storage: 'setting'
		},
		{
			label: 'Non-prod instance',
			description:
				'Whether we should consider the reported usage of this instance as non-prod. <a href="https://www.windmill.dev/docs/advanced/instance_settings#non-prod-instance">Learn more</a>',
			key: 'dev_instance',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
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
		},
		{
			label: 'Delete logs from s3 periodically',
			description:
				'Job and service logs are periodically deleted from disk. When this setting is on, they will also be deleted from the object storage.',
			key: 'monitor_logs_on_s3',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		},

		{
			label: 'Instance object storage',
			description:
				' S3/Azure bucket to store large logs and global cache for Python and Go. <a href="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#instance-object-storage">Learn more</a>',
			key: 'object_store_cache_config',
			fieldType: 'object_store_config',
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
		},
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
			label: 'App workspace prefix',
			description:
				'When enabled apps will be accessible at /a/{workspace_id}/{custom_path} instead of /a/{custom_path} allowing you to define same custom path for apps in different workspace without conflict',
			key: 'app_workspaced_route',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		}
	],
	SMTP: [
		{
			label: 'SMTP',
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
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'UV extra index url',
			description: 'Add private extra Pip registry',
			key: 'pip_extra_index_url',
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Npm config registry',
			description: 'Add private npm registry',
			key: 'npm_config_registry',
			fieldType: 'text',
			placeholder: 'https://registry.npmjs.org/:_authToken=npm_FOOBAR',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Bunfig install scopes',
			description:
				'Add private scoped registries for Bun, See: https://bun.sh/docs/install/registries',
			key: 'bunfig_install_scopes',
			fieldType: 'text',
			placeholder: '"@myorg3" = { token = "mytoken", url = "https://registry.myorg.com/" }',
			storage: 'setting',
			ee_only: ''
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
			fieldType: 'text',
			placeholder: 'https://user:password@artifacts.foo.com/maven',
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
			fieldType: 'text',
			placeholder: 'https://user:password@gems.foo.com/',
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
			description: 'Add private PowerShell repository Personal Access Token',
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
			ee_only: 'Full text search across jobs and service logs is an EE feature'
		}
	],

	Telemetry: [
		{
			label: 'Disable telemetry',
			key: 'disable_stats',
			fieldType: 'boolean',
			storage: 'setting'
		}
	]
}

export const settingsKeys = Object.keys(settings)
