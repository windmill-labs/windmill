import { spawn } from 'node:child_process'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
	formatFrontendBenchmarkProgressEvent,
	parseFrontendBenchmarkProgressLine
} from './progress'
import type { BenchmarkRunResult } from '../../core/types'

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url))
const FRONTEND_DIR = path.join(REPO_ROOT, 'frontend')
const FRONTEND_BENCHMARK_TEST = '../ai_evals/adapters/frontend/vitestAdapter.test.ts'
const FRONTEND_BENCHMARK_CONFIG = '../ai_evals/adapters/frontend/vitest.config.ts'

export type FrontendMode = 'flow' | 'app' | 'script'

export async function runFrontendBenchmarkAdapter(input: {
	mode: FrontendMode
	caseIds: string[]
	runs: number
	model?: string
	verbose?: boolean
	backendValidation?: string
}): Promise<BenchmarkRunResult> {
	const tempDir = await mkdtemp(path.join(tmpdir(), 'wmill-frontend-benchmark-'))
	const outputPath = path.join(tempDir, 'result.json')

	try {
		await runVitestBenchmark(
			path.join(FRONTEND_DIR, 'node_modules', '.bin', 'vitest'),
			[
				'run',
				FRONTEND_BENCHMARK_TEST,
				'--project',
				'server',
				'--config',
				FRONTEND_BENCHMARK_CONFIG
			],
			{
				cwd: FRONTEND_DIR,
				env: {
					...process.env,
					BROWSERSLIST_IGNORE_OLD_DATA: '1',
					WMILL_FRONTEND_AI_EVAL_OUTPUT_PATH: outputPath,
					WMILL_FRONTEND_AI_EVAL_MODE: input.mode,
					WMILL_FRONTEND_AI_EVAL_CASE_IDS: JSON.stringify(input.caseIds),
					WMILL_FRONTEND_AI_EVAL_RUNS: String(input.runs),
					WMILL_FRONTEND_AI_EVAL_MODEL: input.model ?? "",
					WMILL_FRONTEND_AI_EVAL_PROGRESS: '1',
					WMILL_FRONTEND_AI_EVAL_VERBOSE: input.verbose ? '1' : '0',
					WMILL_FRONTEND_AI_EVAL_BACKEND_VALIDATION: input.backendValidation ?? ''
				}
			}
		)

		const raw = await readFile(outputPath, 'utf8')
		return JSON.parse(raw) as BenchmarkRunResult
	} catch (error) {
		throw new Error(`Frontend benchmark adapter failed:\n${toErrorMessage(error)}`)
	} finally {
		await rm(tempDir, { recursive: true, force: true })
	}
}

async function runVitestBenchmark(
	command: string,
	args: string[],
	options: {
		cwd: string
		env: NodeJS.ProcessEnv
	}
): Promise<void> {
	const child = spawn(command, args, {
		cwd: options.cwd,
		env: options.env,
		stdio: ['ignore', 'pipe', 'pipe']
	})

	let stdout = ''
	let stderr = ''
	let stderrLineBuffer = ''
	let assistantStreamOpen = false

	child.stdout?.setEncoding('utf8')
	child.stdout?.on('data', (chunk: string) => {
		stdout += chunk
	})

	child.stderr?.setEncoding('utf8')
	child.stderr?.on('data', (chunk: string) => {
		stderrLineBuffer += chunk
		const { remainder, passthrough, nextAssistantStreamOpen } = drainProgressLines(
			stderrLineBuffer,
			assistantStreamOpen
		)
		stderrLineBuffer = remainder
		stderr += passthrough
		assistantStreamOpen = nextAssistantStreamOpen
	})

	await new Promise<void>((resolve, reject) => {
		child.on('error', reject)
		child.on('close', (code) => {
			if (stderrLineBuffer.length > 0) {
				const {
					remainder,
					passthrough,
					nextAssistantStreamOpen
				} = drainProgressLines(`${stderrLineBuffer}\n`, assistantStreamOpen)
				stderrLineBuffer = remainder
				stderr += passthrough
				assistantStreamOpen = nextAssistantStreamOpen
			}

			if (code === 0) {
				if (assistantStreamOpen) {
					process.stderr.write('\n')
				}
				resolve()
				return
			}

			const details = [`vitest exited with code ${code}`, stdout, stderr].filter(Boolean).join('\n')
			reject(new Error(details))
		})
	})
}

function drainProgressLines(
	buffer: string,
	initialAssistantStreamOpen: boolean
): {
	remainder: string
	passthrough: string
	nextAssistantStreamOpen: boolean
} {
	let remainder = buffer
	let passthrough = ''
	let assistantStreamOpen = initialAssistantStreamOpen

	while (true) {
		const newlineIndex = remainder.indexOf('\n')
		if (newlineIndex === -1) {
			return { remainder, passthrough, nextAssistantStreamOpen: assistantStreamOpen }
		}

		const line = remainder.slice(0, newlineIndex).replace(/\r$/, '')
		remainder = remainder.slice(newlineIndex + 1)

		const progressEvent = parseFrontendBenchmarkProgressLine(line)
		if (progressEvent) {
			if (progressEvent.type === 'assistant-message-start') {
				if (assistantStreamOpen) {
					process.stderr.write('\n')
				}
				process.stderr.write(
					`${formatCasePrefix(progressEvent.caseNumber, progressEvent.totalCases)} ${progressEvent.caseId} attempt ${progressEvent.attempt}/${progressEvent.runs} assistant:\n`
				)
				assistantStreamOpen = true
				continue
			}

			if (progressEvent.type === 'assistant-chunk') {
				process.stderr.write(progressEvent.chunk)
				continue
			}

			if (progressEvent.type === 'assistant-message-end') {
				if (assistantStreamOpen) {
					process.stderr.write('\n')
				}
				assistantStreamOpen = false
				continue
			}

			if (assistantStreamOpen) {
				process.stderr.write('\n')
				assistantStreamOpen = false
			}
			process.stderr.write(`${formatFrontendBenchmarkProgressEvent(progressEvent)}\n`)
			continue
		}

		if (shouldSuppressFrontendStderrLine(line)) {
			continue
		}

		passthrough += `${line}\n`
		process.stderr.write(`${line}\n`)
	}
}

function formatCasePrefix(caseNumber: number, totalCases: number): string {
	return `[${caseNumber}/${totalCases}]`
}

function shouldSuppressFrontendStderrLine(line: string): boolean {
	return (
		line.startsWith('[baseline-browser-mapping] ') ||
		line.startsWith('Browserslist: browsers data (caniuse-lite) is ') ||
		line.includes('update-browserslist-db@latest') ||
		line.includes('update-db#readme')
	)
}

function toErrorMessage(error: unknown): string {
	if (error instanceof Error) {
		return error.message
	}
	return String(error)
}
