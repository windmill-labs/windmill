import { describe, expect, it } from 'vitest'
import { isWorkflowAsCode } from './wacToFlow'

const tsWac = `
import { workflow, task } from "windmill-client"

const process = task(async () => "ok")

export const main = workflow(async () => await process())
`

describe('isWorkflowAsCode', () => {
	it('detects WAC only for Bun TypeScript and Python', () => {
		expect(isWorkflowAsCode(tsWac, 'bun')).toBe(true)
		expect(isWorkflowAsCode(tsWac, 'deno')).toBe(false)
		expect(isWorkflowAsCode(tsWac, 'nativets')).toBe(false)
		expect(isWorkflowAsCode('@workflow\nasync def main():\n    return None\n', 'python3')).toBe(true)
	})
})
