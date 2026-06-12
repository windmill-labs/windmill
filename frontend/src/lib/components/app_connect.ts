export const apiTokenApps: Record<
	string,
	{ img?: string; instructions: string[]; linkedSecret?: string }
> = {
	airtable: {
		img: '/airtable_connect.png',
		instructions: [
			'Go to <a href="https://airtable.com/create/tokens" target="_blank" rel=”noopener noreferrer”>https://airtable.com/create/tokens</a>',
			'Click on "Create new token"',
			'Set a name, specify the scopes or the access level and click on "Create token"',
			'Copy the token'
		]
	},
	discord_webhook: {
		img: '/discord_webhook.png',
		instructions: ['Click on Server Settings', 'Click on Integration', 'Find "Webhooks"'],
		linkedSecret: 'webhook_url'
	},
	toggl: {
		img: '/toggl_connect.png',
		instructions: [
			'Go to <a href="https://track.toggl.com/profile" target="_blank" rel=”noopener noreferrer”>https://track.toggl.com/profile</a>',
			'Find "API Token"'
		]
	},
	mailchimp: {
		img: '/mailchimp_connect.png',
		instructions: [
			'Go to <a href="https://admin.mailchimp.com/account/api" target="_blank" rel=”noopener noreferrer”>https://admin.mailchimp.com/account/api</a>',
			'Find "Your API Keys"'
		]
	},
	sendgrid: {
		img: '/sendgrid_connect.png',
		instructions: [
			'Go to <a href="https://app.sendgrid.com/settings/api_keys" target="_blank" rel=”noopener noreferrer”>https://app.sendgrid.com/settings/api_keys</a>',
			'Create an API key',
			'Copy your key'
		]
	},
	supabase: {
		img: '/supabase_connect.png',
		instructions: ['Go to the API Settings of your app to find the project URL and key']
	},

	square: {
		img: '/square_connect.gif',
		instructions: [
			'Go to <a href="https://developer.squareup.com/apps" target="_blank" rel=”noopener noreferrer”>https://developer.squareup.com/apps</a>',
			'In the left pane, choose Credentials',
			'At the top of the page, choose Production mode for a production access token or Sandbox mode for a Sandbox access token.'
		]
	}
}

export function linkedSecretValue(x: string) {
	let r = 0
	let lowerCasedX = x.toLowerCase()
	if (lowerCasedX.includes('secret')) {
		r += 10
	}
	if (lowerCasedX.includes('password')) {
		r += 5
	}
	if (lowerCasedX.includes('private')) {
		r += 4
	}
	if (lowerCasedX.includes('key')) {
		r += 3
	}
	if (lowerCasedX.includes('token')) {
		r += 2
	}
	if (lowerCasedX.includes('pass')) {
		r += 1
	}
	return r
}

export function forceSecretValue(resourceType: string): string | undefined {
	if (resourceType == 'git_repository') {
		return 'url'
	}
}

export interface ClientCredentialsFields {
	clientIdField: string
	clientSecretField: string
	tokenField: string
	tokenUrlField?: string
}

/**
 * Detect whether a resource type supports the OAuth client-credentials flow,
 * based on its schema carrying real credential fields.
 *
 * Property names are matched after normalization (lowercased, `_`/`-`
 * stripped) so `client_id`, `clientId` and `azureClientId` all qualify. The
 * flow is offered only when the match is unambiguous: exactly one
 * client-id-ish field, exactly one client-secret-ish field and a `token`
 * field (home of the platform-managed access token). An optional unique
 * token-URL-ish field is used to store the token endpoint on the resource.
 */
export function detectClientCredentials(
	schema: any
): ClientCredentialsFields | undefined {
	const props = schema?.properties
	if (!props || typeof props !== 'object') {
		return undefined
	}
	const normalize = (s: string) => s.toLowerCase().replace(/[_-]/g, '')
	const stringKeys = Object.keys(props).filter((k) => props[k]?.type === 'string')

	const clientIds = stringKeys.filter((k) => normalize(k).endsWith('clientid'))
	const clientSecrets = stringKeys.filter((k) => normalize(k).endsWith('clientsecret'))
	const tokens = stringKeys.filter((k) => normalize(k) === 'token')
	const tokenUrls = stringKeys.filter((k) => normalize(k).endsWith('tokenurl'))

	if (clientIds.length !== 1 || clientSecrets.length !== 1 || tokens.length !== 1) {
		return undefined
	}
	return {
		clientIdField: clientIds[0],
		clientSecretField: clientSecrets[0],
		tokenField: tokens[0],
		tokenUrlField: tokenUrls.length === 1 ? tokenUrls[0] : undefined
	}
}
