import { readdirSync } from 'node:fs'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

// Renaming the page back drops it into the (logged) layout, where it hangs on
// "Loading user..." with no type error and no other failing test — see the page header.
const routeDir = dirname(fileURLToPath(import.meta.url))

describe('mcp oauth consent route', () => {
	it('escapes the (logged) layout', () => {
		expect(readdirSync(routeDir)).toContain('+page@(root).svelte')
	})
})
