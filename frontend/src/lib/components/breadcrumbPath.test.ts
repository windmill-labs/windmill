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

	it('gives up on a path that names nothing yet, trailing separator included', () => {
		// The editor binds the path live, so every keystroke reaches the breadcrumb.
		expect(splitItemPath('f/demo')).toBeUndefined()
		expect(splitItemPath('f/demo/')).toBeUndefined()
	})
})
