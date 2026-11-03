// The MCP server settles a run's arguments in Rust (`ai_session_tools/runs.rs`), read against
// the same cases by `run_args_match_the_shared_cases`.
import { readFileSync } from 'node:fs'
import { describe, expect, it, vi } from 'vitest'

const stored: string[] = []

vi.mock('$lib/gen', () => ({
	VariableService: {
		createVariable: vi.fn(async ({ requestBody }: any) => {
			stored.push(requestBody.value)
			return requestBody.path
		})
	}
}))

vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { workspaceStore: writable('test-ws'), userStore: writable({ username: 'ada' }) }
})

import {
	dropUndeclaredArgs,
	enforceDisabledDefaults,
	redactFileArgs,
	redactSecretArgs
} from './job_args'
import { processSecretArgs } from './secretArgUtils'

const { cases } = JSON.parse(
	readFileSync(
		new URL(
			'../../../../backend/windmill-api/src/ai_session_tools/run_args_cases.json',
			import.meta.url
		),
		'utf8'
	)
)

describe('run arguments, against the cases the server shares', () => {
	for (const c of cases) {
		it(c.name, async () => {
			if (c.declared) {
				const { args, undeclaredKeys } = dropUndeclaredArgs(c.args, c.schema)
				expect(args).toEqual(c.declared.args)
				expect(undeclaredKeys).toEqual(c.declared.undeclared)
			}
			if (c.disabled_defaults) {
				const { args, resetKeys } = enforceDisabledDefaults(c.args, c.schema)
				expect(args).toEqual(c.disabled_defaults.args)
				expect(resetKeys).toEqual(c.disabled_defaults.reset)
			}
			if (c.secrets) {
				stored.length = 0
				const out = await processSecretArgs(c.args, c.schema, 'test-ws')
				const named = JSON.parse(
					JSON.stringify(out).replace(/u\/ada\/secret_arg\/[A-Za-z0-9]+/g, '<minted>')
				)
				expect(named).toEqual(c.secrets.args)
				expect([...stored].sort()).toEqual(c.secrets.stored)
			}
			if (c.redacted) {
				expect(redactFileArgs(redactSecretArgs(c.args, c.schema), c.schema)).toEqual(c.redacted)
			}
		})
	}
})
