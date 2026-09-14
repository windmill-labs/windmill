import { SvelteMap } from 'svelte/reactivity'

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

/**
 * Names for the items no rule names (`gsheets` -> Google Sheets): stored with resource types,
 * and curated by the hub for integrations. Unnamed items are left to the word table above on
 * purpose. Reactive because they fill in after first render: a label already on screen updates
 * when they land.
 */
const resourceTypeNames = new SvelteMap<string, string>()
const hubIntegrationNames = new SvelteMap<string, string>()

type Named = { name: string; display_name?: string | null }

/**
 * Updates only the entries it is given: one without a name drops any name kept for it, which is
 * how a hub predating display names reverts to inferred labels. Entries it is not given are left
 * alone on purpose, since callers pass single rows and `kind`-filtered listings.
 */
function recordNames(names: SvelteMap<string, string>, entries: Named[]): void {
	for (const entry of entries) {
		const label = entry.display_name?.trim()
		if (label) names.set(entry.name, label)
		else names.delete(entry.name)
	}
}

export function setResourceTypeDisplayNames(types: Named[]): void {
	recordNames(resourceTypeNames, types)
}

export function setHubIntegrationDisplayNames(integrations: Named[]): void {
	recordNames(hubIntegrationNames, integrations)
}

/** The prefix the resources page puts on a type created in a workspace. */
const CUSTOM_TYPE_PREFIX = 'c_'

export function isCustomResourceTypeName(name: string): boolean {
	return name.startsWith(CUSTOM_TYPE_PREFIX)
}

/**
 * Display name for a resource type: `gsheets` -> `Google Sheets` where the type stores that name,
 * otherwise inferred from the type name (`mysql` -> `MySQL`, `c_acme_api` -> `Acme API`), the
 * word table above covering what capitalizing gets wrong.
 */
export function resourceTypeDisplayName(name: string): string {
	return (
		resourceTypeNames.get(name) ??
		titleize(isCustomResourceTypeName(name) ? name.slice(CUSTOM_TYPE_PREFIX.length) : name)
	)
}

/**
 * Display name for a hub integration slug, as the hub pickers label their filters:
 * `activecampaign` -> `ActiveCampaign` from a hub that names it, `Activecampaign` from one
 * that does not.
 */
export function integrationDisplayName(app: string): string {
	return hubIntegrationNames.get(app) ?? titleize(app)
}

/**
 * Lowercases each word before the lookup and splits on `-` too: a private hub can predate the
 * slug rule, and still serve `aws-ses` or `RSS`. `Object.hasOwn`, because a word like
 * `constructor` would otherwise resolve up the object literal's prototype chain.
 */
function titleize(name: string): string {
	return name
		.split(/[_-]/)
		.filter(Boolean)
		.map((word) => {
			const lower = word.toLowerCase()
			return Object.hasOwn(RESOURCE_TYPE_WORDS, lower)
				? RESOURCE_TYPE_WORDS[lower]
				: word.charAt(0).toUpperCase() + word.slice(1)
		})
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
