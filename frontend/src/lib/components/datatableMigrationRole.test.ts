import { describe, test, expect } from 'vitest'
import { parseMigrationRole, withMigrationRole } from './datatableMigrationRole'

describe('parseMigrationRole', () => {
	test('reads an annotation from the leading comment block', () => {
		expect(parseMigrationRole('-- role analyst\nBEGIN;\nSELECT 1;\nEND;')).toBe('analyst')
		expect(parseMigrationRole('\n\n-- role read-only_1\nBEGIN;')).toBe('read-only_1')
	})

	test('stops at the first non-comment line', () => {
		// Inside the transaction it is prose, not a directive — and the executor
		// would not read it either.
		expect(parseMigrationRole('BEGIN;\n-- role analyst\nSELECT 1;')).toBeUndefined()
	})

	test('ignores prose that merely starts with role', () => {
		expect(parseMigrationRole('-- role based access is handled below\nBEGIN;')).toBeUndefined()
		expect(parseMigrationRole('-- role\nBEGIN;')).toBeUndefined()
		expect(parseMigrationRole('-- roles analyst\nBEGIN;')).toBeUndefined()
		expect(parseMigrationRole('-- role bad;name\nBEGIN;')).toBeUndefined()
	})

	test('no annotation means the default role', () => {
		expect(parseMigrationRole('BEGIN;\nSELECT 1;\nEND;')).toBeUndefined()
	})
})

describe('withMigrationRole', () => {
	test('prepends above everything, so the executor still parses it', () => {
		const out = withMigrationRole('BEGIN;\nSELECT 1;\nEND;', 'analyst')
		expect(out.split('\n')[0]).toBe('-- role analyst')
		expect(parseMigrationRole(out)).toBe('analyst')
	})

	test('replaces rather than stacks annotations', () => {
		const once = withMigrationRole('BEGIN;', 'analyst')
		const twice = withMigrationRole(once, 'auditor')
		expect(twice.match(/-- role /g)?.length).toBe(1)
		expect(parseMigrationRole(twice)).toBe('auditor')
	})

	test('undefined strips the annotation back to the default role', () => {
		const stripped = withMigrationRole(withMigrationRole('BEGIN;\nEND;', 'analyst'), undefined)
		expect(parseMigrationRole(stripped)).toBeUndefined()
		expect(stripped).toBe('BEGIN;\nEND;')
	})

	test('leaves other leading comments alone', () => {
		const out = withMigrationRole('-- adds an index\nBEGIN;', 'analyst')
		expect(out).toBe('-- role analyst\n-- adds an index\nBEGIN;')
	})

	test('round-trips through the select both ways', () => {
		let code = 'BEGIN;\nCREATE TABLE t(x int);\nEND;'
		for (const role of ['analyst', 'auditor', undefined, 'admin']) {
			code = withMigrationRole(code, role)
			expect(parseMigrationRole(code)).toBe(role)
		}
	})
})
