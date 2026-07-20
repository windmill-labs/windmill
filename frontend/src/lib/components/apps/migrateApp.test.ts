import { describe, expect, it } from 'vitest'
import { migrateApp } from './migrateApp'
import type { App } from './types'

describe('migrateApp', () => {
	it('defaults a missing grid so the grid components never dereference undefined', () => {
		const app = { subgrids: {} } as unknown as App

		migrateApp(app)

		expect(app.grid).toEqual([])
	})
})
