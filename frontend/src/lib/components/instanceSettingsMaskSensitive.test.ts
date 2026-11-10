import { describe, it, expect } from 'vitest'
import { maskSensitive, SENSITIVE_CHANGED, SENSITIVE_UNCHANGED } from './instanceSettings'

const initialValues = {
	jwt_secret: 'old-jwt',
	license_key: 'old-license',
	base_url: 'http://a',
	smtp_settings: { smtp_host: 'h', smtp_password: 'old-smtp' },
	external_secrets: { vault: { token: 'old-vault' } },
	object_store_cache_config: { serviceAccountKey: { k: 'old' } },
	workspace_registries: { pip: { url: 'https://user:pw@old' } }
}
const initialOauths = { github: { id: 'gid', secret: 'old-oauth' } }

function clone<T>(v: T): T {
	return JSON.parse(JSON.stringify(v))
}

// The Review changes diff is a screen-shareable view: no secret may appear in clear
// on either side, while a changed secret must still read as changed.
describe('maskSensitive with markChanged', () => {
	const modifiedValues = {
		...clone(initialValues),
		jwt_secret: 'new-jwt',
		base_url: 'http://b',
		smtp_settings: { smtp_host: 'h', smtp_password: 'new-smtp' },
		external_secrets: { vault: { token: 'new-vault' } },
		object_store_cache_config: { serviceAccountKey: { k: 'new' } },
		oauths: { github: { id: 'gid', secret: 'new-oauth' } }
	}

	it('hides every secret on the original side as unchanged', () => {
		const out = maskSensitive(
			{ ...clone(initialValues), oauths: clone(initialOauths) },
			initialValues,
			initialOauths,
			{ markChanged: true }
		)
		const text = JSON.stringify(out)
		for (const s of ['old-jwt', 'old-license', 'old-smtp', 'old-vault', 'old-oauth', 'pw@old']) {
			expect(text).not.toContain(s)
		}
		expect(out.jwt_secret).toBe(SENSITIVE_UNCHANGED)
		expect(out.workspace_registries).toBe(SENSITIVE_UNCHANGED)
		expect(out.base_url).toBe('http://a')
	})

	it('marks changed secrets without printing old or new values', () => {
		const out = maskSensitive(modifiedValues, initialValues, initialOauths, { markChanged: true })
		const text = JSON.stringify(out)
		for (const s of ['jwt', 'license', 'smtp', 'vault', 'oauth']) {
			expect(text).not.toContain(`old-${s}`)
			expect(text).not.toContain(`new-${s}`)
		}
		expect(text).not.toContain('pw@old')
		expect(out.jwt_secret).toBe(SENSITIVE_CHANGED)
		expect(out.license_key).toBe(SENSITIVE_UNCHANGED)
		expect(out.workspace_registries).toBe(SENSITIVE_UNCHANGED)
		expect(out.smtp_settings).toEqual({ smtp_host: 'h', smtp_password: SENSITIVE_CHANGED })
		expect(out.external_secrets.vault.token).toBe(SENSITIVE_CHANGED)
		expect(out.object_store_cache_config.serviceAccountKey).toBe(SENSITIVE_CHANGED)
		expect(out.oauths.github).toEqual({ id: 'gid', secret: SENSITIVE_CHANGED })
		expect(out.base_url).toBe('http://b')
	})

	it('does not mutate the values it masks', () => {
		const vals = clone(modifiedValues)
		maskSensitive(vals, initialValues, initialOauths, { markChanged: true })
		expect(vals).toEqual(modifiedValues)
	})
})

// The YAML editor restores SENSITIVE_UNCHANGED on parse, so a changed secret must stay
// editable in clear there.
describe('maskSensitive without markChanged', () => {
	it('keeps changed secrets in clear and masks unchanged ones', () => {
		const out = maskSensitive(
			{ jwt_secret: 'new-jwt', license_key: 'old-license' },
			initialValues,
			initialOauths
		)
		expect(out).toEqual({ jwt_secret: 'new-jwt', license_key: SENSITIVE_UNCHANGED })
	})
})
