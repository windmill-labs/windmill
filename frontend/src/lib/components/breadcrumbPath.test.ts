import { describe, it, expect } from 'vitest'
import { splitItemPath } from './breadcrumbPath'

describe('splitItemPath', () => {
	it('walks the scope, each subfolder, then the item, each with its cumulative path', () => {
		expect(splitItemPath('f/demo/sub/weather_report')).toEqual({
			dirs: [
				{ name: 'f/demo', fullPath: 'f/demo' },
				{ name: 'sub', fullPath: 'f/demo/sub' }
			],
			leaf: { name: 'weather_report', fullPath: 'f/demo/sub/weather_report' }
		})
	})

	it('gives up before there is a scope', () => {
		expect(splitItemPath('f/demo')).toBeUndefined()
	})

	it('keeps the folders while the name is still being typed', () => {
		// The editor binds the path live, so every keystroke reaches the breadcrumb: the trail must
		// not drop out and come back on the first character of the name.
		expect(splitItemPath('f/demo/')).toEqual({
			dirs: [{ name: 'f/demo', fullPath: 'f/demo' }],
			leaf: undefined
		})
	})
})
