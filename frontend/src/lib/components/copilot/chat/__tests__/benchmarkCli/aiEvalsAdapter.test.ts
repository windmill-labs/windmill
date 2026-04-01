import { expect, it, vi } from 'vitest'
// @ts-ignore - Node.js fs/promises
import { mkdir, writeFile } from 'fs/promises'
// @ts-ignore - Node.js path
import { dirname, resolve } from 'path'

vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: {
		parse: (value: string) => ({ toString: () => value })
	},
	MarkerSeverity: {
		Error: 8,
		Warning: 4,
		Info: 2,
		Hint: 1
	}
}))

vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))

vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({
	default: () => ({})
}))

vi.mock('$lib/components/vscode', () => ({}))

const benchmarkOutputPath = process.env.WMILL_FRONTEND_AI_EVAL_OUTPUT_PATH
const benchmarkIt = benchmarkOutputPath ? it : it.skip

benchmarkIt(
	'runs the frontend benchmark adapter from environment input',
	async () => {
		const { runFrontendBenchmarkFromEnv } = await import('./frontendBenchmarkRunner')
		const payload = await runFrontendBenchmarkFromEnv()
		const absoluteOutputPath = resolve(benchmarkOutputPath!)
		await mkdir(dirname(absoluteOutputPath), { recursive: true })
		await writeFile(absoluteOutputPath, JSON.stringify(payload, null, 2) + '\n', 'utf8')

		expect(payload.variants.length).toBeGreaterThan(0)
	},
	600_000
)
