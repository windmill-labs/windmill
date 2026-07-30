import { describe, it, expect } from 'vitest'
import { isValidWebhookBaseUrl } from './instanceSettings'

/**
 * Kept in lockstep with `webhook_base_url_matches_the_ui_validator` in
 * backend/windmill-common/src/global_settings.rs — the same table on both sides, so
 * a value this field accepts can never be rejected on save, and vice versa. Add
 * cases to both or neither.
 */
const ACCEPTED = [
	'https://hooks.example.com',
	'http://hooks.example.com:8080',
	'https://example.com/windmill',
	' https://hooks.example.com ',
	'https://[::1]:8000'
]

const REJECTED = [
	'httpss://hooks.example.com',
	'hooks.example.com',
	'ftp://hooks.example.com',
	'https://',
	'https://hooks.example.com?token=x',
	'https://hooks.example.com#frag',
	'https://hooks example.com',
	'https://hooks.example.com/',
	'https://hooks.example.com:abc',
	'https://x/a b',
	'https://hooks.example.com?',
	'https://hooks.example.com#',
	'https://user:password@hooks.example.com',
	'https://user@hooks.example.com'
]

describe('isValidWebhookBaseUrl', () => {
	it.each(ACCEPTED)('accepts %j', (value) => {
		expect(isValidWebhookBaseUrl(value)).toBe(true)
	})

	it.each(REJECTED)('rejects %j', (value) => {
		expect(isValidWebhookBaseUrl(value)).toBe(false)
	})

	it.each([true, 42, {}, [], null] as unknown[])('rejects the non-string %j without throwing', (value) => {
		// `Setting.isValid` is typed `any` and YAML mode can supply any JSON shape.
		expect(isValidWebhookBaseUrl(value as never)).toBe(value === null)
	})

	it('treats unset and blank as valid, since the setting is optional', () => {
		expect(isValidWebhookBaseUrl(undefined)).toBe(true)
		expect(isValidWebhookBaseUrl('')).toBe(true)
		expect(isValidWebhookBaseUrl('   ')).toBe(true)
	})
})
