import { describe, expect, it } from 'vitest'
import { roleAfterDatatableChange } from './datatableUtils.svelte'

describe('roleAfterDatatableChange', () => {
	it('drops a role that belongs to the data table being left', () => {
		// The role exists on `main` and says nothing about `second`, so an app
		// whose default moves must not keep naming it.
		expect(roleAfterDatatableChange('main', 'second', 'analyst')).toBe(undefined)
		expect(roleAfterDatatableChange('main', undefined, 'analyst')).toBe(undefined)
	})

	it('keeps it when only the schema moved', () => {
		expect(roleAfterDatatableChange('main', 'main', 'analyst')).toBe('analyst')
		expect(roleAfterDatatableChange(undefined, undefined, undefined)).toBe(undefined)
	})
})
