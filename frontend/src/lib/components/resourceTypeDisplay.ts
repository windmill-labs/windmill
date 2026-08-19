/**
 * Resource types are named by abbreviation — `gdrive`, `gcal`, `s3` — so a product's real
 * name ("Google Drive", "Amazon S3") only ever appears in its description. Matching on the
 * name alone means searching "google" finds none of the Google integrations.
 */
export function resourceTypeSearchText(name: string, description?: string): string {
	const label = resourceTypeDisplayName(name)
	const named = label.toLowerCase() === name ? name : `${name} ${label}`
	return description ? `${named} ${description}` : named
}

function isWordStart(haystack: string, at: number): boolean {
	return at === 0 || !/[a-z0-9]/.test(haystack[at - 1])
}

/**
 * Sort key for one resource type against a search query, lowest first.
 *
 * Three fields can match, and they are not worth the same: the name someone types to get
 * this exact type, the product name it is displayed under, and the description — which
 * mentions a dozen products in passing, so `googleai` has to outrank `anthropic` on
 * "google". Hence the tiers: name, then display name, then description.
 * Within a field, ties break on where the match starts: a field opening with the query is
 * about the query, one mentioning it halfway through is an aside.
 *
 * Returns Number.MAX_SAFE_INTEGER when the query appears in no field, so a caller matching
 * more loosely than a substring (uFuzzy) keeps those results last instead of dropping them.
 */
export function resourceTypeMatchRank(
	name: string,
	description: string | undefined,
	query: string
): number {
	const q = query.trim().toLowerCase()
	if (q === '') return 0

	const n = name.toLowerCase()
	if (n === q) return 0

	const fields: [string, number][] = [
		[n, 1],
		[resourceTypeDisplayName(name).toLowerCase(), 4],
		[(description ?? '').toLowerCase(), 7]
	]

	for (const [haystack, baseTier] of fields) {
		const at = haystack.indexOf(q)
		if (at < 0) continue
		const tier = at === 0 ? baseTier : isWordStart(haystack, at) ? baseTier + 1 : baseTier + 2
		return tier * 1e4 + Math.min(at, 9999)
	}

	return Number.MAX_SAFE_INTEGER
}

/**
 * Rank-sorts resource types by how well they match `query`, keeping the incoming order
 * for equal ranks (and for an empty query, where callers rely on their own ordering).
 */
export function sortResourceTypesByMatch<T>(
	items: T[],
	query: string,
	name: (item: T) => string,
	description: (item: T) => string | undefined
): T[] {
	if (query.trim() === '') return items
	return items
		.map((item, index) => ({
			item,
			index,
			rank: resourceTypeMatchRank(name(item), description(item), query)
		}))
		.sort((a, b) => a.rank - b.rank || a.index - b.index)
		.map((entry) => entry.item)
}

/**
 * Casing for name parts that capitalizing the first letter gets wrong — acronyms, and brands
 * that carry a capital inside the word. Anything absent is capitalized, which is right for
 * the great majority of types (`stripe` -> `Stripe`).
 */
const RESOURCE_TYPE_WORDS: Record<string, string> = {
	ai: 'AI',
	ai21: 'AI21',
	amqp: 'AMQP',
	api: 'API',
	aws: 'AWS',
	cms: 'CMS',
	crm: 'CRM',
	db: 'DB',
	ftp: 'FTP',
	gcp: 'GCP',
	gpg: 'GPG',
	hr: 'HR',
	http: 'HTTP',
	id: 'ID',
	ifs: 'IFS',
	json: 'JSON',
	jwt: 'JWT',
	ldap: 'LDAP',
	mcp: 'MCP',
	mqtt: 'MQTT',
	ms: 'Microsoft',
	nats: 'NATS',
	oauth: 'OAuth',
	odk: 'ODK',
	oidc: 'OIDC',
	rss: 'RSS',
	s3: 'S3',
	sdk: 'SDK',
	smtp: 'SMTP',
	sql: 'SQL',
	ssh: 'SSH',
	url: 'URL',
	ynab: 'YNAB',
	abstractapi: 'AbstractAPI',
	arcgis: 'ArcGIS',
	assemblyai: 'AssemblyAI',
	bigquery: 'BigQuery',
	chromadb: 'ChromaDB',
	circleci: 'CircleCI',
	clickhouse: 'ClickHouse',
	clickup: 'ClickUp',
	cockroachdb: 'CockroachDB',
	comapeo: 'CoMapeo',
	convertkit: 'ConvertKit',
	currencyapi: 'CurrencyAPI',
	customai: 'CustomAI',
	datocms: 'DatoCMS',
	dbt: 'dbt',
	deepl: 'DeepL',
	deepseek: 'DeepSeek',
	digitalocean: 'DigitalOcean',
	docspring: 'DocSpring',
	docusign: 'DocuSign',
	edgedb: 'EdgeDB',
	faunadb: 'FaunaDB',
	gcloud: 'Google Cloud',
	ghostcms: 'Ghost CMS',
	gitbook: 'GitBook',
	github: 'GitHub',
	gitlab: 'GitLab',
	googleai: 'GoogleAI',
	graphql: 'GraphQL',
	groqai: 'GroqAI',
	hubspot: 'HubSpot',
	ipinfo: 'IPinfo',
	kobotoolbox: 'KoboToolbox',
	leonardoai: 'LeonardoAI',
	linkedin: 'LinkedIn',
	lumaai: 'LumaAI',
	mailerlite: 'MailerLite',
	mongodb: 'MongoDB',
	mysql: 'MySQL',
	netbox: 'NetBox',
	netsuite: 'NetSuite',
	newsapi: 'NewsAPI',
	neondb: 'NeonDB',
	nocodb: 'NocoDB',
	openai: 'OpenAI',
	openrouter: 'OpenRouter',
	oracledb: 'OracleDB',
	pagerduty: 'PagerDuty',
	pandadoc: 'PandaDoc',
	paypal: 'PayPal',
	planetscale: 'PlanetScale',
	postgresql: 'PostgreSQL',
	readme: 'ReadMe',
	rest: 'REST',
	quickbooks: 'QuickBooks',
	sendgrid: 'SendGrid',
	servicenow: 'ServiceNow',
	signoz: 'SigNoz',
	surrealdb: 'SurrealDB',
	togetherai: 'TogetherAI',
	webscrapingai: 'WebScrapingAI',
	weatherapi: 'WeatherAPI',
	whatsapp: 'WhatsApp',
	woocommerce: 'WooCommerce'
}

/** Types whose label is not their name with the parts re-cased. */
const RESOURCE_TYPE_NAMES: Record<string, string> = {
	adobe_acrobat_sign: 'Adobe Acrobat Sign',
	bamboo_hr: 'BambooHR',
	cacertificate: 'CA certificate',
	deep_infra: 'DeepInfra',
	gcal: 'Google Calendar',
	gdocs: 'Google Docs',
	gdrive: 'Google Drive',
	gforms: 'Google Forms',
	gsheets: 'Google Sheets',
	gworkspace: 'Google Workspace',
	ms_sql_server: 'Microsoft SQL Server',
	sage_intacct: 'Sage Intacct',
	sensortower: 'Sensor Tower',
	their_stack: 'TheirStack'
}

/** The prefix the resources page puts on a type created in a workspace. */
const CUSTOM_TYPE_PREFIX = 'c_'

export function isCustomResourceTypeName(name: string): boolean {
	return name.startsWith(CUSTOM_TYPE_PREFIX)
}

/**
 * Display name for a resource type: `adobe_acrobat_sign` -> `Adobe Acrobat Sign`, `mysql` ->
 * `MySQL`, `c_acme_api` -> `Acme API`. Inferred from the name, since nothing in the type
 * carries a product name — the two tables above only cover what the inference gets wrong.
 */
export function resourceTypeDisplayName(name: string): string {
	const exact = RESOURCE_TYPE_NAMES[name]
	if (exact) return exact
	const stripped = isCustomResourceTypeName(name) ? name.slice(CUSTOM_TYPE_PREFIX.length) : name
	return stripped
		.split('_')
		.map((word) => RESOURCE_TYPE_WORDS[word] ?? word.charAt(0).toUpperCase() + word.slice(1))
		.join(' ')
}

/**
 * Drawer title for creating a resource, named after its type once one is picked.
 *
 * Article-free on purpose: whether a label takes "a" or "an" follows how it is said, which
 * the spelling does not carry — "an S3" but "a NATS", "an MCP" but "a REST" — so every rule
 * over the type name gets a class of them wrong.
 */
export function addResourceTitle(resourceType: string | undefined): string {
	return resourceType ? `Add ${resourceTypeDisplayName(resourceType)} resource` : 'Add a resource'
}
