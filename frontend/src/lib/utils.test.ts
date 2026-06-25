import { describe, it, expect } from 'vitest'
import {
	cleanValueProperties,
	getQueryStmtCountHeuristic,
	parseDbInputFromAssetSyntax
} from './utils'

describe('parseDbInputFromAssetSyntax', () => {
	it('parses a table path', () => {
		expect(parseDbInputFromAssetSyntax('ducklake://main/orders')).toEqual({
			type: 'ducklake',
			ducklake: 'main',
			specificTable: 'orders',
			specificSchema: undefined
		})
	})

	it('parses a schema-qualified table path', () => {
		expect(parseDbInputFromAssetSyntax('ducklake://main/analytics.orders')).toEqual({
			type: 'ducklake',
			ducklake: 'main',
			specificTable: 'orders',
			specificSchema: 'analytics'
		})
	})

	it('handles a catalog-only path without throwing (no table segment)', () => {
		// e.g. `// materialize ducklake` → `ducklake://main` — must not throw.
		expect(parseDbInputFromAssetSyntax('ducklake://main')).toEqual({
			type: 'ducklake',
			ducklake: 'main',
			specificTable: undefined,
			specificSchema: undefined
		})
	})
})

describe('getQueryStmtCountHeuristic', () => {
	describe('basic statements', () => {
		it('should count a single statement without semicolon', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users')).toBe(1)
		})

		it('should count a single statement with semicolon', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users;')).toBe(1)
		})

		it('should count multiple statements', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users; SELECT * FROM posts;')).toBe(2)
		})

		it('should handle empty string as implicit statement', () => {
			expect(getQueryStmtCountHeuristic('')).toBe(0)
		})

		it('should handle whitespace only as implicit statement', () => {
			expect(getQueryStmtCountHeuristic('   \n\t  ')).toBe(0)
		})

		it('should handle statement with trailing whitespace', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users;   \n  ')).toBe(1)
		})

		it('should count three statements', () => {
			expect(
				getQueryStmtCountHeuristic('SELECT 1; UPDATE users SET name = "test"; DELETE FROM logs;')
			).toBe(3)
		})
	})

	describe('single-quoted strings', () => {
		it('should ignore semicolon in single-quoted string', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'hello;world' FROM users")).toBe(1)
		})

		it('should handle escaped single quote', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'it''s working' FROM users")).toBe(1)
		})

		it('should handle multiple escaped single quotes', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'it''s a ''test''' FROM users")).toBe(1)
		})

		it('should handle semicolon in escaped quote context', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'val''s;data' FROM users")).toBe(1)
		})

		it('should handle multiple statements with single-quoted strings', () => {
			expect(
				getQueryStmtCountHeuristic("SELECT 'hello;' FROM users; SELECT 'world;' FROM posts")
			).toBe(2)
		})

		it('should handle adjacent escaped quotes', () => {
			expect(getQueryStmtCountHeuristic("SELECT '''' FROM users")).toBe(1)
		})
	})

	describe('double-quoted strings', () => {
		it('should ignore semicolon in double-quoted string', () => {
			expect(getQueryStmtCountHeuristic('SELECT "hello;world" FROM users')).toBe(1)
		})

		it('should handle escaped double quote', () => {
			expect(getQueryStmtCountHeuristic('SELECT "it""s working" FROM users')).toBe(1)
		})

		it('should handle multiple escaped double quotes', () => {
			expect(getQueryStmtCountHeuristic('SELECT "it""s a ""test""" FROM users')).toBe(1)
		})

		it('should handle semicolon in escaped quote context', () => {
			expect(getQueryStmtCountHeuristic('SELECT "val""s;data" FROM users')).toBe(1)
		})

		it('should handle adjacent escaped quotes', () => {
			expect(getQueryStmtCountHeuristic('SELECT """" FROM users')).toBe(1)
		})
	})

	describe('mixed quotes', () => {
		it('should handle both single and double quotes in same query', () => {
			expect(getQueryStmtCountHeuristic('SELECT "col;1", \'val;2\' FROM users')).toBe(1)
		})

		it('should handle single quote inside double quote', () => {
			expect(getQueryStmtCountHeuristic('SELECT "it\'s;ok" FROM users')).toBe(1)
		})

		it('should handle double quote inside single quote', () => {
			expect(getQueryStmtCountHeuristic('SELECT \'"quoted";value\' FROM users')).toBe(1)
		})

		it('should handle complex nested scenarios', () => {
			expect(
				getQueryStmtCountHeuristic('SELECT "a\'b;c", \'d"e;f\', g FROM users; DELETE FROM logs')
			).toBe(2)
		})
	})

	describe('line comments', () => {
		it('should ignore semicolon in line comment', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 -- comment;here\nFROM users')).toBe(1)
		})

		it('should handle multiple line comments', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 -- comment;1\n-- comment;2\nFROM users')).toBe(1)
		})

		it('should handle line comment at end of query', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users -- final;comment')).toBe(1)
		})

		it('should handle line comment with semicolon at end', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users; -- comment;here')).toBe(1)
		})

		it('should count statements after line comment', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1; -- comment\nSELECT 2')).toBe(2)
		})

		it('should handle line comment without newline at end (stays in comment state)', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 FROM users -- comment;here')).toBe(1)
		})

		it('should handle dashes inside string not as comment', () => {
			expect(getQueryStmtCountHeuristic("SELECT '--not;comment' FROM users")).toBe(1)
		})

		it('should handle single dash (not a comment)', () => {
			expect(getQueryStmtCountHeuristic('SELECT a-b FROM users')).toBe(1)
		})
	})

	describe('block comments', () => {
		it('should ignore semicolon in block comment', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 /* comment;here */ FROM users')).toBe(1)
		})

		it('should handle multiline block comment', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 /* comment;\nacross;\nlines */ FROM users')).toBe(
				1
			)
		})

		it('should handle multiple block comments', () => {
			expect(
				getQueryStmtCountHeuristic(
					'SELECT /* c1;here */ 1, /* c2;here */ 2 FROM /* c3;here */ users'
				)
			).toBe(1)
		})

		it('should handle nested-looking block comments', () => {
			expect(
				getQueryStmtCountHeuristic('SELECT 1 /* outer /* inner;here */ still */ FROM users')
			).toBe(1)
		})

		it('should handle block comment at end', () => {
			expect(getQueryStmtCountHeuristic('SELECT * FROM users /* final;comment */')).toBe(1)
		})

		it('should count statements after block comment', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1; /* comment */ SELECT 2')).toBe(2)
		})

		it('should handle block comment markers in string', () => {
			expect(getQueryStmtCountHeuristic("SELECT '/* not;comment */' FROM users")).toBe(1)
		})

		it('should handle unclosed block comment (stays in comment state)', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1 /* unclosed;comment')).toBe(1)
		})

		it('should handle slash-star inside string not as comment', () => {
			expect(getQueryStmtCountHeuristic("SELECT '/*;not comment' FROM users")).toBe(1)
		})
	})

	describe('mixed comments and strings', () => {
		it('should handle comment after string', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'value;1' -- comment;2\nFROM users")).toBe(1)
		})

		it('should handle string after comment', () => {
			expect(getQueryStmtCountHeuristic("SELECT 1 -- comment\n, 'value;here' FROM users")).toBe(1)
		})

		it('should handle complex mix', () => {
			expect(
				getQueryStmtCountHeuristic(
					'SELECT "col;1" /* comment;1 */ , \'val;2\' -- comment;2\nFROM users'
				)
			).toBe(1)
		})

		it('should handle quotes in comments', () => {
			expect(getQueryStmtCountHeuristic("SELECT 1 /* it's;working */ FROM users")).toBe(1)
		})
		it('should handle comment markers in string', () => {
			expect(getQueryStmtCountHeuristic("SELECT '--/*;*/' FROM users -- real;comment")).toBe(1)
		})
	})

	describe('edge cases', () => {
		it('should handle only semicolons', () => {
			expect(getQueryStmtCountHeuristic(';;;')).toBe(3)
		})

		it('should handle semicolons with whitespace', () => {
			expect(getQueryStmtCountHeuristic(' ; ; ; ')).toBe(3)
		})

		it('should handle query ending with multiple semicolons', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1;;')).toBe(2)
		})

		it('should handle unclosed single quote (stays in quote state)', () => {
			expect(getQueryStmtCountHeuristic("SELECT 'unclosed")).toBe(1)
		})

		it('should handle unclosed double quote (stays in quote state)', () => {
			expect(getQueryStmtCountHeuristic('SELECT "unclosed')).toBe(1)
		})

		it('should handle unclosed line comment (no newline at end)', () => {
			expect(getQueryStmtCountHeuristic('SELECT 1; -- comment;here')).toBe(1)
		})

		it('should handle all types together', () => {
			expect(
				getQueryStmtCountHeuristic(
					`SELECT "col;1", 'val;2' FROM users WHERE x = 1; -- comment;here
/* block;comment */ UPDATE posts SET title = 'new;title';
DELETE FROM logs -- final;cleanup`
				)
			).toBe(3)
		})

		it('should handle empty statements', () => {
			expect(getQueryStmtCountHeuristic(';')).toBe(1)
		})

		it('should handle consecutive escaped quotes at statement boundary', () => {
			expect(getQueryStmtCountHeuristic("SELECT '''';")).toBe(1)
		})

		it('should handle alternating quote types', () => {
			expect(getQueryStmtCountHeuristic('SELECT "a", \'b\', "c", \'d\' FROM users')).toBe(1)
		})

		it('should handle very long string with semicolons', () => {
			const longString = ';'.repeat(1000)
			expect(getQueryStmtCountHeuristic(`SELECT '${longString}' FROM users`)).toBe(1)
		})

		it('should handle realistic multiline query', () => {
			const query = `
SELECT
  id,
  name,
  "user;email" -- email column
FROM users
WHERE status = 'active;pending'
  AND created_at > '2024-01-01';

/* Update user preferences */
UPDATE user_settings
SET theme = 'dark;mode'
WHERE user_id IN (SELECT id FROM users WHERE name LIKE '%test%');

-- Cleanup old data
DELETE FROM logs WHERE timestamp < NOW() - INTERVAL '30 days'
			`
			expect(getQueryStmtCountHeuristic(query)).toBe(3)
		})
	})
})

describe('cleanValueProperties', () => {
	const serverManagedKeys = [
		'parent_hash',
		'hash',
		'assets',
		'inherited_labels',
		'draft',
		'draft_only',
		'draft_saved_at',
		'draft_created_at',
		'is_draft',
		'other_drafts_users',
		'created_at',
		'created_by',
		'workspace_id',
		'parent_hashes',
		'lock_error_logs'
	]

	it('strips every server-managed bookkeeping key', () => {
		const input: any = { summary: 'hi' }
		for (const key of serverManagedKeys) {
			input[key] = 'noise'
		}
		const cleaned = cleanValueProperties(input) as any
		for (const key of serverManagedKeys) {
			expect(cleaned).not.toHaveProperty(key)
		}
	})

	it('preserves user-editable keys', () => {
		const input: any = {
			summary: 'my script',
			description: 'does things',
			content: 'export function main() {}',
			schema: { properties: { x: { type: 'string' } } },
			language: 'bun',
			created_at: '2024-01-01'
		}
		const cleaned = cleanValueProperties(input) as any
		expect(cleaned.summary).toBe('my script')
		expect(cleaned.description).toBe('does things')
		expect(cleaned.content).toBe('export function main() {}')
		expect(cleaned.schema).toEqual({ properties: { x: { type: 'string' } } })
		expect(cleaned.language).toBe('bun')
		expect(cleaned).not.toHaveProperty('created_at')
	})

	it('preserves lock so version-to-version diffs still surface lockfile changes', () => {
		const cleaned = cleanValueProperties({ summary: 'hi', lock: 'resolved deps' } as any) as any
		expect(cleaned.lock).toBe('resolved deps')
	})

	it('preserves extra_perms so folder workspace/fork diffs still surface permission changes', () => {
		const cleaned = cleanValueProperties({
			summary: 'hi',
			extra_perms: { 'u/foo': true }
		} as any) as any
		expect(cleaned.extra_perms).toEqual({ 'u/foo': true })
	})

	it('returns non-object values unchanged', () => {
		expect(cleanValueProperties('hello' as any)).toBe('hello')
		expect(cleanValueProperties(42 as any)).toBe(42)
	})

	it('does not mutate the input object', () => {
		const input: any = { summary: 'hi', created_at: '2024-01-01' }
		cleanValueProperties(input)
		expect(input).toHaveProperty('created_at')
	})
})
