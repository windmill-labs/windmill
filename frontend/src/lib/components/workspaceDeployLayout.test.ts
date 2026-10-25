import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const layoutSource = readFileSync(
	join(dirname(fileURLToPath(import.meta.url)), 'WorkspaceDeployLayout.svelte'),
	'utf8'
)

describe('WorkspaceDeployLayout deploy drawer scrolling', () => {
	it('sizes the list against the drawer flex column instead of h-full', () => {
		expect(layoutSource).toContain('flex flex-col flex-1 min-h-0')
		expect(layoutSource).toContain('flex-1 min-h-0 overflow-y-auto')
	})
})
