export interface Setting {
	label: string
	description?: string
	placeholder?: string
	cloudonly?: boolean
	ee_only?: string
	tooltip?: string
	key: string
	fieldType:
		| 'text'
		| 'number'
		| 'boolean'
		| 'password'
		| 'select'
		| 'textarea'
		| 'seconds'
		| 'email'
		| 'license_key'
		| 'object_store_config'
		| 'critical_error_channels'
		| 'slack_connect'
		| 'smtp_connect'
	storage: SettingStorage
	advancedToggle?: {
		label: string
		onChange: (values: Record<string, any>) => Record<string, any>
		checked: (values: Record<string, any>) => boolean
	}
	hiddenIfNull?: boolean
	requiresReloadOnChange?: boolean
	isValid?: (value: any) => boolean
	error?: string
	defaultValue?: () => any
}

export type SettingStorage = 'setting'

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
				value
					? value?.startsWith('http') &&
					  value.includes('://') &&
					  !value?.endsWith('/') &&
					  !value?.endsWith(' ')
					: false
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
		},
		{
			label: 'Expose metrics',
			description:
				'Expose Prometheus metrics for workers and servers on port 8001 at /metrics. <a href="https://www.windmill.dev/docs/advanced/instance_settings#expose-metrics">Learn more</a>',
			key: 'expose_metrics',
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
			label: 'Critical alert channels',
			description:
				'Channels to send critical alerts to. SMTP must be configured for the email channel. A Slack workspace must be connected to the instance for the Slack channel. <a href="https://www.windmill.dev/docs/core_concepts/critical_alert_channels">Learn more</a>',
			key: 'critical_error_channels',
			fieldType: 'critical_error_channels',
			storage: 'setting',
			ee_only: 'Channels other than tracing are only available in the EE version'
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
			label: 'Azure OpenAI base path',
			description:
				'All Windmill AI features will run on the specified deployed model. Format: https://{your-resource-name}.openai.azure.com/openai/deployments/{deployment-id}. <a href="https://www.windmill.dev/docs/core_concepts/ai_generation#azure-openai-advanced-models">Learn more</a>',
			key: 'openai_azure_base_path',
			fieldType: 'text',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Private Hub base url',
			description:
				'Base URL of your private Hub instance, without trailing slash. <a href="https://www.windmill.dev/docs/core_concepts/private_hub">Learn more</a>',
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
		}
	],
	'SSO/OAuth': [],
	Registries: [
		{
			label: 'Pip Index Url',
			description: 'Add private Pip registry',
			key: 'pip_index_url',
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Pip Extra Index Url',
			description: 'Add private extra Pip registry',
			key: 'pip_extra_index_url',
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Npm Config Registry',
			description: 'Add private npm registry',
			key: 'npm_config_registry',
			fieldType: 'text',
			placeholder: 'https://registry.npmjs.org/:_authToken=npm_FOOBAR',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Bunfig Install Scopes',
			description:
				'Add private scoped registries for Bun, See: https://bun.sh/docs/install/registries',
			key: 'bunfig_install_scopes',
			fieldType: 'text',
			placeholder: '"@myorg3" = { token = "mytoken", url = "https://registry.myorg.com/" }',
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
	Slack: [
		{
			label: 'Slack',
			key: 'slack',
			fieldType: 'slack_connect',
			storage: 'setting',
			ee_only: ''
		}
	],
	'SCIM/SAML': [
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
	],
	Debug: [
		{
			label: 'Keep job directories',
			key: 'keep_job_dir',
			fieldType: 'boolean',
			description: 'Keep Job directories after execution at /tmp/windmill/WORKER/JOB_ID',
			storage: 'setting'
		},
		{
			label: 'Expose debug metrics',
			key: 'expose_debug_metrics',
			fieldType: 'boolean',
			description: 'Expose additional metrics (require metrics to be enabled)',
			storage: 'setting'
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
