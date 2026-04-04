import { execFile as execFileCallback } from 'node:child_process'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { promisify } from 'node:util'
import { fileURLToPath } from 'node:url'

const execFile = promisify(execFileCallback)
const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FRONTEND_DIR = path.join(REPO_ROOT, 'frontend')
const FRONTEND_BENCHMARK_TEST = '../ai_evals/adapters/frontend/vitestAdapter.test.ts'

export type FrontendSurfaceName = 'frontend-flow' | 'frontend-app' | 'frontend-script'

export interface FrontendAdapterAttempt {
	attempt: number
	passed: boolean
	durationMs: number
	assistantMessageCount: number
	toolCallCount: number
	toolsUsed: string[]
	checks: Array<{ name: string; passed: boolean; required?: boolean }>
	requiredFailedChecks: string[]
	judgeScore: number | null
	judgeStatement: string | null
	error: string | null
}

export interface FrontendAdapterCaseResult {
	caseId: string
	attempts: FrontendAdapterAttempt[]
}

export interface FrontendAdapterVariantResult {
	variant: string
	provider: string
	model: string
	judgeModel: string | null
	caseResults: FrontendAdapterCaseResult[]
}

export interface FrontendAdapterPayload {
	surface: 'flow' | 'app' | 'script'
	runs: number
	variants: FrontendAdapterVariantResult[]
}

export async function runFrontendBenchmarkAdapter(input: {
	surface: FrontendSurfaceName
	caseIds: string[]
	variantIds: string[]
	runs: number
}): Promise<FrontendAdapterPayload> {
	const tempDir = await mkdtemp(path.join(tmpdir(), 'wmill-frontend-benchmark-'))
	const outputPath = path.join(tempDir, 'result.json')

	try {
		await execFile(
			path.join(FRONTEND_DIR, 'node_modules', '.bin', 'vitest'),
			[
				'run',
				FRONTEND_BENCHMARK_TEST,
				'--project',
				'server',
				'--config',
				'vite.config.js'
			],
			{
				cwd: FRONTEND_DIR,
				env: {
					...process.env,
					WMILL_FRONTEND_AI_EVAL_OUTPUT_PATH: outputPath,
					WMILL_FRONTEND_AI_EVAL_SURFACE: frontendSurfaceToAdapterSurface(input.surface),
					WMILL_FRONTEND_AI_EVAL_CASE_IDS: JSON.stringify(input.caseIds),
					WMILL_FRONTEND_AI_EVAL_VARIANT_IDS: JSON.stringify(input.variantIds),
					WMILL_FRONTEND_AI_EVAL_RUNS: String(input.runs)
				},
				maxBuffer: 10 * 1024 * 1024
			}
		)

		const raw = await readFile(outputPath, 'utf8')
		return JSON.parse(raw) as FrontendAdapterPayload
	} catch (error) {
		const executionError = error as Error & { stdout?: string; stderr?: string }
		const details = [executionError.message, executionError.stdout, executionError.stderr]
			.filter(Boolean)
			.join('\n')
		throw new Error(`Frontend benchmark adapter failed:\n${details}`)
	} finally {
		await rm(tempDir, { recursive: true, force: true })
	}
}

function frontendSurfaceToAdapterSurface(surface: FrontendSurfaceName): 'flow' | 'app' | 'script' {
	if (surface === 'frontend-flow') {
		return 'flow'
	}
	if (surface === 'frontend-app') {
		return 'app'
	}
	return 'script'
}
