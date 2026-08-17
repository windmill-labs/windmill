import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { isValidLogoutRedirect, toSameOriginRelativePath } from './logoutRedirect'

describe('isValidLogoutRedirect', () => {
	beforeEach(() => {
		vi.stubGlobal('window', { location: { origin: 'http://localhost:3000' } })
	})

	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('accepts same-origin absolute URLs', () => {
		expect(isValidLogoutRedirect('http://localhost:3000/')).toBe(true)
		expect(isValidLogoutRedirect('http://localhost:3000/runs?workspace=foo')).toBe(true)
	})

	it('accepts root-relative paths', () => {
		expect(isValidLogoutRedirect('/foo')).toBe(true)
	})

	it('accepts windmill.dev hosts', () => {
		expect(isValidLogoutRedirect('https://app.windmill.dev/foo')).toBe(true)
	})

	it('rejects cross-origin and protocol-relative', () => {
		expect(isValidLogoutRedirect('https://evil.com/')).toBe(false)
		expect(isValidLogoutRedirect('//evil.com')).toBe(false)
	})
})

describe('toSameOriginRelativePath', () => {
	beforeEach(() => {
		// The function reads `window.location.origin` to resolve relative URLs
		// and detect cross-origin targets. The default test env is `node`, so
		// stub a minimal `window` with the origin we want to anchor against.
		vi.stubGlobal('window', { location: { origin: 'http://localhost:3000' } })
	})

	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('returns same-origin relative paths as-is', () => {
		expect(toSameOriginRelativePath('/a/nucleus/chat')).toBe('/a/nucleus/chat')
		expect(toSameOriginRelativePath('/runs?workspace=foo')).toBe('/runs?workspace=foo')
		expect(toSameOriginRelativePath('/path#fragment')).toBe('/path#fragment')
	})

	it('reduces same-origin full URLs to relative paths', () => {
		expect(toSameOriginRelativePath('http://localhost:3000/a/project-template/test')).toBe(
			'/a/project-template/test'
		)
		expect(
			toSameOriginRelativePath('http://localhost:3000/a/project-template/test?foo=bar#baz')
		).toBe('/a/project-template/test?foo=bar#baz')
	})

	it('rejects cross-origin URLs', () => {
		expect(toSameOriginRelativePath('https://evil.com/path')).toBeNull()
		expect(toSameOriginRelativePath('http://other.windmill.dev/path')).toBeNull()
	})

	it('rejects protocol-relative and back-slash tricks', () => {
		expect(toSameOriginRelativePath('//evil.com')).toBeNull()
		expect(toSameOriginRelativePath('/\\evil.com')).toBeNull()
	})

	it('rejects empty and null inputs', () => {
		expect(toSameOriginRelativePath('')).toBeNull()
		expect(toSameOriginRelativePath(null)).toBeNull()
		expect(toSameOriginRelativePath(undefined)).toBeNull()
	})

	it('rejects values containing control characters', () => {
		expect(toSameOriginRelativePath('/path\r\nSet-Cookie: x=y')).toBeNull()
		expect(toSameOriginRelativePath('/path\x00null')).toBeNull()
	})

	it('resolves rootless inputs against the current origin', () => {
		// Relative-style inputs without a leading slash are not what callers
		// pass us in practice, but resolving them against the current origin
		// is a safe outcome (still same-origin) — better than silently
		// dropping them.
		expect(toSameOriginRelativePath('relative/no/slash')).toBe('/relative/no/slash')
	})

	it('rejects values that exceed the length cap', () => {
		expect(toSameOriginRelativePath('/' + 'a'.repeat(2048))).toBeNull()
	})

	it('handles the SP BASE_URL with no path as same-origin root', () => {
		// When SAML SP libraries send the bare BASE_URL as RelayState default,
		// reducing it to the origin's root path is still safe and same-origin.
		expect(toSameOriginRelativePath('http://localhost:3000')).toBe('/')
		expect(toSameOriginRelativePath('http://localhost:3000/')).toBe('/')
	})
})
