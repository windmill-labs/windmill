import { stripSandboxSuffix } from './oauthRegistry'

/** Hand-picked sections for the resource types people reach for most, in the order the add
 * resource drawer renders them. `popular` holds the handful picked most of the time and is
 * listed ahead of every full section; `keys` holds the rest of the category. Everything not
 * listed falls under "Others". Keys are hub resource type names; a key the instance has not
 * synced simply renders nothing. */
export const RESOURCE_TYPE_CATEGORIES: {
	title: string
	/** Names the rest of the category on the link from its popular section. */
	others: string
	popular: string[]
	keys: string[]
}[] = [
	{
		title: 'Databases',
		others: 'other databases',
		popular: [
			'postgresql',
			'mysql',
			'ms_sql_server',
			'bigquery',
			'snowflake',
			'mongodb',
			'supabase',
			'neondb'
		],
		keys: [
			'snowflake_oauth',
			'oracledb',
			'redshift',
			'clickhouse',
			'databricks',
			'cockroachdb',
			'planetscale',
			'turso',
			'firebase',
			'surrealdb'
		]
	},
	{
		title: 'AI',
		others: 'other AI providers',
		popular: ['openai', 'anthropic', 'googleai', 'mistral', 'deepseek', 'groq'],
		keys: [
			'azure_openai',
			'aws_bedrock',
			'azure_foundry',
			'openrouter',
			'togetherai',
			'cohere',
			'deep_infra',
			'customai',
			'pinecone',
			'chromadb'
		]
	},
	{
		title: 'Object storage',
		others: 'other object storage',
		popular: ['s3', 'azure_blob', 'gcloud_storage'],
		keys: ['s3_aws_oidc']
	},
	{
		title: 'Cloud providers',
		others: 'other cloud providers',
		popular: ['aws', 'gcloud', 'gcp_service_account', 'azure'],
		keys: ['aws_oidc', 'azure_oauth', 'azure_workload_identity', 'cloudflare', 'digitalocean']
	},
	{
		title: 'Messaging & email',
		others: 'other messaging & email',
		popular: ['slack', 'ms_teams_webhook', 'smtp', 'gmail', 'discord_webhook', 'telegram'],
		keys: [
			'discord_bot_configuration',
			'whatsapp_business',
			'twilio',
			'sendgrid',
			'resend',
			'mailgun'
		]
	},
	{
		title: 'Event streaming',
		others: 'other event streaming',
		popular: ['kafka', 'nats', 'mqtt'],
		keys: ['amqp']
	},
	{
		title: 'Productivity',
		others: 'other productivity tools',
		popular: ['gsheets', 'gdrive', 'notion', 'airtable', 'jira'],
		keys: ['gcal', 'gdocs', 'confluence', 'linear', 'asana', 'trello', 'clickup']
	},
	{
		title: 'Developer tools',
		others: 'other developer tools',
		popular: ['github', 'gitlab'],
		keys: ['github_app', 'bitbucket', 'git_repository', 'sentry', 'datadog']
	},
	{
		title: 'CRM & payments',
		others: 'other CRM & payments',
		popular: ['hubspot', 'salesforce', 'stripe'],
		keys: ['shopify', 'zendesk']
	}
]

type ResourceTypeCategory = { title: string; popular: boolean }

const categoryByKey = new Map<string, ResourceTypeCategory>(
	RESOURCE_TYPE_CATEGORIES.flatMap(({ title, popular, keys }) => [
		...popular.map((key): [string, ResourceTypeCategory] => [key, { title, popular: true }]),
		...keys.map((key): [string, ResourceTypeCategory] => [key, { title, popular: false }])
	])
)

/** A sandbox client (`salesforce_sandbox`) files under its parent's section. */
export function resourceTypeCategory(key: string): ResourceTypeCategory | undefined {
	return categoryByKey.get(stripSandboxSuffix(key))
}
