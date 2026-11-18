import { describe, test, expect } from 'vitest'
import { parseMigrationRole, withMigrationRole } from './datatableMigrationRole'

describe('parseMigrationRole', () => {
	test('reads every spelling the server accepts from the leading comment block', () => {
		for (const line of [
			'-- role analyst',
			'-- Role: analyst',
			'-- role=analyst',
			'-- role analyst;'
		]) {
			expect(parseMigrationRole(`\n${line}\nBEGIN;\nEND;`)).toEqual({
				kind: 'role',
				role: 'analyst'
			})
		}
	})

	test('an annotation below BEGIN is not one', () => {
		expect(parseMigrationRole('BEGIN;\n-- role analyst\nEND;')).toEqual({ kind: 'none' })
	})

	test('a malformed attempt is an error, not the default', () => {
		for (const line of [
			'-- role based access below',
			'-- role',
			'-- role:',
			'-- role an;alytics'
		]) {
			expect(parseMigrationRole(`${line}\nBEGIN;`)).toEqual({ kind: 'malformed', line })
		}
	})

	test('comments that do not start with the word role are ignored', () => {
		expect(parseMigrationRole('-- roles analyst\n-- rolex\nBEGIN;')).toEqual({ kind: 'none' })
	})
})

describe('withMigrationRole', () => {
	test('leads above BEGIN, so the server reads it', () => {
		const out = withMigrationRole('BEGIN;\nSELECT 1;\nEND;', 'analyst')
		expect(out).toBe('-- role analyst\nBEGIN;\nSELECT 1;\nEND;')
	})

	test('replaces any attempt rather than stacking, malformed ones included', () => {
		const out = withMigrationRole(
			'-- Role: auditor\n-- role oops no\n-- keep me\nBEGIN;',
			'analyst'
		)
		expect(out).toBe('-- role analyst\n-- keep me\nBEGIN;')
	})

	test('undefined strips the annotation, so it runs as admin', () => {
		expect(withMigrationRole('-- role analyst\n\nBEGIN;\nEND;', undefined)).toBe('BEGIN;\nEND;')
	})

	test('refuses a name the server would refuse', () => {
		expect(() => withMigrationRole('BEGIN;', 'bad;name')).toThrow()
	})
})
