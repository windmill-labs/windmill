import { describe, it, expect } from 'vitest'
import { canonicalModulePath, findModulePathClash } from './scriptModulePath'

describe('canonicalModulePath', () => {
	// The pair the duplicate and reserved-name checks exist to catch: both
	// spellings resolve to the same file in the job directory.
	it('rewrites a redundant spelling to the file it names', () => {
		expect(canonicalModulePath('./dbt_project.yml')).toEqual({ path: 'dbt_project.yml' })
		expect(canonicalModulePath('models//x.sql')).toEqual({ path: 'models/x.sql' })
		expect(canonicalModulePath('  ./models/./sub//x.sql  ')).toEqual({
			path: 'models/sub/x.sql'
		})
		expect(canonicalModulePath('models/x.sql')).toEqual({ path: 'models/x.sql' })
	})

	it('refuses a path that leaves the bundle', () => {
		expect(canonicalModulePath('../x.sql')).toHaveProperty('error')
		expect(canonicalModulePath('models/../../x.sql')).toHaveProperty('error')
		expect(canonicalModulePath('/etc/x.sql')).toHaveProperty('error')
		expect(canonicalModulePath('./')).toHaveProperty('error')
	})

	// Matches the worker's own rule: `..` is traversal only as a whole segment.
	it('takes dots inside a name as part of the name', () => {
		expect(canonicalModulePath('models/weird..name.sql')).toEqual({
			path: 'models/weird..name.sql'
		})
	})
})

describe('findModulePathClash', () => {
	// A bundle pushed by the CLI can hold a non-canonical key, so the clash has to
	// be found from either side, and named the way the tree shows it.
	it('finds a key that resolves to the same file, however either is spelled', () => {
		const modules = { './dbt_project.yml': {}, 'models/x.sql': {} }
		expect(findModulePathClash(modules, 'dbt_project.yml')).toBe('./dbt_project.yml')
		expect(findModulePathClash(modules, 'models/x.sql')).toBe('models/x.sql')
		expect(findModulePathClash(modules, 'models/y.sql')).toBeUndefined()
		expect(findModulePathClash(undefined, 'models/x.sql')).toBeUndefined()
	})

	// The worker does not trim path components, so an imported `x.sql ` is its
	// own file and must not stand in the way of adding `x.sql`.
	it('does not fold a key whose name carries whitespace', () => {
		expect(findModulePathClash({ 'models/x.sql ': {} }, 'models/x.sql')).toBeUndefined()
	})

	// A rename must not stop at the module being renamed: with both spellings in
	// the bundle, that would hide the other one and overwrite its content.
	it('keeps looking past the key being renamed', () => {
		const modules = { './models/x.sql': {}, 'models/x.sql': {} }
		expect(findModulePathClash(modules, 'models/x.sql', './models/x.sql')).toBe('models/x.sql')
		expect(
			findModulePathClash({ './models/x.sql': {} }, 'models/x.sql', './models/x.sql')
		).toBeUndefined()
	})
})
