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
	storage: SettingStorage
	isValid?: (value: any) => boolean
	error?: string
	defaultValue?: () => any
}

export type SettingStorage = 'setting' | 'config'

export const settings: Record<string, Setting[]> = {
	Core: [
		{
			label: 'Base Url',
			description: 'Public base url of the instance',
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
			description: 'Domain to display in webhooks for email triggers (should match the MX record)',
			key: 'email_domain',
			fieldType: 'text',
			storage: 'setting',
			placeholder: 'mail.windmill.com'
		},
		{
			label: 'Request Size Limit In MB',
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
			description: 'Default timeout for individual jobs',
			fieldType: 'seconds',
			storage: 'setting',
			cloudonly: false
		},
		{
			label: 'Max Timeout for sync endpoints',
			key: 'timeout_wait_result',
			cloudonly: true,
			fieldType: 'seconds',
			placeholder: '60',
			storage: 'config'
		},
		{
			label: 'License Key',
			description: 'License Key required to use the EE (switch image for windmill-ee)',
			key: 'license_key',
			fieldType: 'license_key',
			placeholder: 'only needed to prepare upgrade to EE',
			storage: 'setting'
		},
		{
			label: 'Non-prod instance',
			description: 'Whether we should consider the reported usage of this instance as non-prod',
			key: 'dev_instance',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: 'This is only relevant for EE'
		},
		{
			label: 'Retention Period in secs',
			key: 'retention_period_secs',
			description: 'How long to keep the jobs data in the database (max 30 days on CE)',
			fieldType: 'seconds',
			placeholder: '30',
			storage: 'setting',
			ee_only: 'You can only adjust this setting to above 30 days in the EE version',
			cloudonly: false
		},
		{
			label: 'Expose metrics',
			description: 'Expose prometheus metrics for workers and servers on port 8001 at /metrics',
			key: 'expose_metrics',
			fieldType: 'boolean',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'S3/Azure for Python/Go Cache & Large Logs',
			description: 'Bucket to store large logs and global cache for Python and Go.',
			key: 'object_store_cache_config',
			fieldType: 'object_store_config',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Critical Error Channels',
			description:
				'Channels to send critical errors to. SMTP must be configured for the email channel.',
			key: 'critical_error_channels',
			fieldType: 'critical_error_channels',
			storage: 'setting',
			ee_only: 'Channels other than tracing are only available in the EE version'
		},
		{
			label: 'Azure OpenAI base path',
			description:
				'All Windmill AI features will run on the specified deployed model. Format: https://{your-resource-name}.openai.azure.com/openai/deployments/{deployment-id}',
			key: 'openai_azure_base_path',
			fieldType: 'text',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Private hub base url',
			description: 'Base url of your private hub instance, without trailing slash',
			placeholder: 'https://hub.company.com',
			key: 'hub_base_url',
			fieldType: 'text',
			storage: 'setting',
			ee_only: ''
		}
	],
	'SSO/OAuth': [],
	Registries: [
		{
			label: 'Pip Index Url',
			description: 'Add private PIP registry',
			key: 'pip_index_url',
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Pip Extra Index Url',
			description: 'Add private extra PIP registry',
			key: 'pip_extra_index_url',
			fieldType: 'text',
			placeholder: 'https://username:password@pypi.company.com/simple',
			storage: 'setting',
			ee_only: ''
		},
		{
			label: 'Npm Config Registry',
			description: 'Add private NPM registry',
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
			label: 'Host',
			key: 'smtp_host',
			fieldType: 'text',
			placeholder: 'smtp.gmail.com',
			storage: 'config'
		},
		{
			label: 'Port',
			key: 'smtp_port',
			fieldType: 'number',
			placeholder: '587',
			storage: 'config'
		},
		{
			label: 'Username',
			key: 'smtp_username',
			fieldType: 'text',
			placeholder: 'ruben@windmill.dev',
			storage: 'config'
		},
		{
			label: 'Password',
			key: 'smtp_password',
			fieldType: 'password',
			storage: 'config'
		},
		{
			label: 'From Address',
			key: 'smtp_from',
			placeholder: 'noreply@windmill.dev',
			fieldType: 'email',
			storage: 'config'
		},
		{
			label: 'Implicit TLS',
			key: 'smtp_tls_implicit',
			fieldType: 'boolean',
			storage: 'config'
		}
	],
	'SCIM/SAML': [
		{
			label: 'SCIM Token',
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
			label: 'Keep Job Directories',
			key: 'keep_job_dir',
			fieldType: 'boolean',
			tooltip: 'Keep Job directories after execution at /tmp/windmill/<worker>/<job_id>',
			storage: 'setting'
		},
		{
			label: 'Expose Debug Metrics',
			key: 'expose_debug_metrics',
			fieldType: 'boolean',
			tooltip: 'Expose additional metrics (require metrics to be enabled)',
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
