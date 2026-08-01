import { describe, expect, it } from 'vitest'
import type { ScriptModule } from '$lib/gen'
import { dbtModelSelector, dbtModulePath, dbtProjectFileKey } from './projectFiles'

const mod = (content: string): ScriptModule => ({ content, language: 'dbt' as any })

const PROJECT = `name: jaffle
model-paths: ["models"]
`

describe('dbtModulePath', () => {
	// The worker resolves `.` and `//` away when it materialises the bundle, so a
	// redundant spelling is a second key for one file on disk and walks straight
	// past a check that compares the typed string.
	it('canonicalises before the reserved-name and duplicate checks', () => {
		expect(dbtModulePath('./models//x.sql', {})).toEqual({ path: 'models/x.sql' })
		expect(dbtModulePath('./wm_dbt.yaml', {})).toEqual({
			error: expect.stringContaining('is the descriptor')
		})
		const bundle = { './models/x.sql': mod('select 1') }
		expect(dbtModulePath('models/x.sql', bundle)).toEqual({
			error: expect.stringContaining('./models/x.sql already exists')
		})
	})

	// A path outside the bundle has no canonical form inside it, and the worker
	// drops it — which would read as a file that was added and never written.
	it('refuses a path that escapes the bundle', () => {
		expect(dbtModulePath('../secrets.sql', {})).toEqual({ error: expect.stringContaining('..') })
		expect(dbtModulePath('/etc/x.sql', {})).toEqual({ error: expect.stringContaining('relative') })
	})

	it('refuses an extension dbt does not read', () => {
		expect(dbtModulePath('models/x.txt', {})).toEqual({
			error: expect.stringContaining('must end with')
		})
	})
})

// Every write path canonicalises, so a read that compares the constant exactly
// would neither find a project imported under a redundant spelling nor protect
// it from deletion.
it('resolves the key a bundle actually holds the project file under', () => {
	expect(dbtProjectFileKey({ './dbt_project.yml': mod(PROJECT) })).toBe('./dbt_project.yml')
	expect(dbtProjectFileKey({})).toBeUndefined()
})

describe('dbtModelSelector', () => {
	// Package-qualified, because a bare leaf name also matches a dependency
	// package's model of the same name.
	it('selects a model under the project’s own model-paths', () => {
		const bundle = { 'dbt_project.yml': mod(PROJECT), 'models/orders.sql': mod('select 1') }
		expect(dbtModelSelector(bundle, 'models/orders.sql')).toBe('orders,package:jaffle')
	})

	// A project may put its models anywhere; a macro or singular test is `.sql`
	// too and is not selectable by name, so those fall back to the whole project.
	it('honours a custom model-paths and skips what is not a model', () => {
		const bundle = {
			'dbt_project.yml': mod('name: jaffle\nmodel-paths: ["transform"]\n'),
			'transform/orders.sql': mod('select 1'),
			'macros/cents.sql': mod('{% macro cents() %}{% endmacro %}')
		}
		expect(dbtModelSelector(bundle, 'transform/orders.sql')).toBe('orders,package:jaffle')
		expect(dbtModelSelector(bundle, 'macros/cents.sql')).toBeUndefined()
	})

	it('finds the project file under a redundant spelling', () => {
		const bundle = { './dbt_project.yml': mod(PROJECT), 'models/orders.sql': mod('select 1') }
		expect(dbtModelSelector(bundle, 'models/orders.sql')).toBe('orders,package:jaffle')
	})
})
