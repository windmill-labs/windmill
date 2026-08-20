// Only messages the login endpoint is known to produce are shown. Anything else — a SQL error
// (which the API also returns as a 400, with the query and a source location), a proxy's HTML
// error page — would otherwise be printed verbatim to an unauthenticated visitor.
const KNOWN_LOGIN_ERRORS = ['Password login is disabled on this instance']

const GENERIC_LOGIN_ERROR = 'Could not sign you in. Please try again.'

export function loginErrorMessage(err: any): string {
	// The API returns errors as plain text, prefixed by their class (e.g. "Bad request: Invalid
	// login"); ApiError.message is only the HTTP status text.
	const raw = typeof err?.body === 'string' ? err.body : err?.body?.error?.message
	const body = typeof raw === 'string' ? raw : ''
	const detail = body.replace(/^(Bad request|Internal|Error): /, '').trim()
	if (detail === 'Invalid login') {
		return 'Invalid email or password.'
	}
	if (err?.status === 429) {
		return 'Too many login attempts. Please try again later.'
	}
	if (KNOWN_LOGIN_ERRORS.includes(detail)) {
		return detail
	}
	return GENERIC_LOGIN_ERROR
}
