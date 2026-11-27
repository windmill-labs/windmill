// Only messages the login endpoint is known to produce are shown. Anything else — a SQL error
// (which the API also returns as a 400, with the query and a source location), a proxy's HTML
// error page — would otherwise be printed verbatim to an unauthenticated visitor.
const KNOWN_LOGIN_ERRORS = ['Password login is disabled on this instance']

/** The password matched and the account asks for its authenticator code: a prompt, not a failure. */
export const TOTP_REQUIRED = 'Two-factor code required'

/** The password matched, the instance requires a second factor and the account has none yet. */
export const TOTP_SETUP_REQUIRED = 'Two-factor setup required'

export const TOTP_INVALID_MESSAGE = 'Invalid authentication code.'

const GENERIC_LOGIN_ERROR = 'Could not sign you in. Please try again.'

// The API returns errors as plain text, prefixed by their class (e.g. "Bad request: Invalid
// login"); ApiError.message is only the HTTP status text.
export function loginErrorDetail(err: any): string {
	const raw = typeof err?.body === 'string' ? err.body : err?.body?.error?.message
	const body = typeof raw === 'string' ? raw : ''
	return body.replace(/^(Bad request|Internal|Error): /, '').trim()
}

export function loginErrorMessage(err: any): string {
	const detail = loginErrorDetail(err)
	if (detail === 'Invalid login') {
		return 'Invalid email or password.'
	}
	if (detail === 'Invalid two-factor code') {
		return TOTP_INVALID_MESSAGE
	}
	if (err?.status === 429) {
		return 'Too many login attempts. Please try again later.'
	}
	if (KNOWN_LOGIN_ERRORS.includes(detail)) {
		return detail
	}
	return GENERIC_LOGIN_ERROR
}
