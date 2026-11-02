import { describe, expect, it, vi } from 'vitest'

const readVariable = (path: string) =>
	`import * as wmill from "windmill-client"\nexport async function main() { return await wmill.getVariable("${path}") }\n`

vi.mock('$lib/gen', () => ({
	AppService: {
		getAppByPath: vi.fn(async () => ({
			value: {
				files: {
					'/App.tsx': `export default () => <div>{wmill.getVariable("f/x/browser_secret")}</div>`,
					'/Db.svelte': `<script>const db = wmill.getResource('f/x/app_db')</script>\n<p>{db}</p>`,
					'/notes.md': `getVariable("f/x/not_code")`
				},
				runnables: {}
			}
		}))
	},
	AssetService: { listAssetsByUsage: vi.fn(async () => [[]]) },
	FlowService: {
		getFlowByPath: vi.fn(async () => ({
			schema: {},
			value: {
				modules: [
					{
						id: 'a',
						value: { type: 'script', path: 'f/x/step', hash: 'old', input_transforms: {} }
					},
					{
						id: 'b',
						value: {
							type: 'rawscript',
							language: 'bun',
							content: '',
							input_transforms: { db: { type: 'static', value: '$res:f/x/db' } }
						}
					}
				]
			}
		}))
	},
	ScriptService: {
		getScriptByHash: vi.fn(async () => ({
			language: 'bun',
			content: readVariable('f/x/pinned_secret'),
			schema: {}
		})),
		getScriptByPath: vi.fn(async () => ({
			language: 'bun',
			content: readVariable('f/x/latest_secret'),
			schema: {}
		}))
	},
	ResourceService: {
		getResource: vi.fn(async ({ path }: { path: string }) => ({
			value:
				path === 'f/x/db'
					? { password: '$var:f/x/db_password', tls: { ca: '$res:f/x/ca' } }
					: path === 'f/x/ca'
						? { cert: '$jsonvar:f/x/ca_cert' }
						: {}
		}))
	}
}))
vi.mock('$lib/infer', () => ({ inferAssets: vi.fn(async () => ({ status: 'ok', assets: [] })) }))

import { detectNeeds } from './detectNeeds'

describe('detectNeeds', () => {
	it('reads the pinned version of a called script and what its resources reference', async () => {
		const needs = await detectNeeds('w', 'flow', 'f/x/flow')
		expect(needs.items.map((i) => `${i.kind}:${i.path}`)).toEqual([
			'resource:f/x/ca',
			'variable:f/x/ca_cert',
			'resource:f/x/db',
			'variable:f/x/db_password',
			'variable:f/x/pinned_secret'
		])
		expect(needs.errors).toEqual([])
	})

	it("reads a full-code app's JSX and Svelte source files", async () => {
		const needs = await detectNeeds('w', 'app', 'f/x/app')
		expect(needs.items.map((i) => `${i.kind}:${i.path}`)).toEqual([
			'resource:f/x/app_db',
			'variable:f/x/browser_secret'
		])
	})
})
