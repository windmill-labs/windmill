// @ts-ignore - Node.js fs
import { existsSync, readFileSync } from 'fs'
// @ts-ignore - Node.js path
import { dirname, join, resolve } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'

export interface FlowEvalCaseManifest {
	id: string
	title: string
	userPrompt: string
	expectedFlowPath: string
	initialFlowPath?: string
	minJudgeScore?: number
}

export interface FlowEvalCase extends FlowEvalCaseManifest {
	expectedFlow: Record<string, unknown>
	initialFlow?: Record<string, any>
}

export interface AppEvalCaseManifest {
	id: string
	title: string
	userPrompt: string
	initialAppFixturePath?: string
	minJudgeScore?: number
}

export interface AppEvalCase extends AppEvalCaseManifest {}

function resolveRepoRoot(): string {
	const currentDir = dirname(fileURLToPath(import.meta.url))
	const repoRoot = resolve(currentDir, '../../../..')

	if (!existsSync(join(repoRoot, 'ai_evals'))) {
		throw new Error(`Could not resolve repo root from: ${currentDir}`)
	}

	return repoRoot
}

function readJsonFile<T>(path: string): T {
	return JSON.parse(readFileSync(path, 'utf-8')) as T
}

export function loadFlowEvalCases(): FlowEvalCase[] {
	const repoRoot = resolveRepoRoot()
	const manifestPath = join(repoRoot, 'ai_evals', 'cases', 'frontend', 'flow.json')
	const manifest = readJsonFile<FlowEvalCaseManifest[]>(manifestPath)

	return manifest.map((testCase) => ({
		...testCase,
		expectedFlow: readJsonFile<Record<string, unknown>>(join(repoRoot, testCase.expectedFlowPath)),
		initialFlow: testCase.initialFlowPath
			? readJsonFile<Record<string, any>>(join(repoRoot, testCase.initialFlowPath))
			: undefined
	}))
}

export function loadAppEvalCases(): AppEvalCase[] {
	const repoRoot = resolveRepoRoot()
	const manifestPath = join(repoRoot, 'ai_evals', 'cases', 'frontend', 'app.json')
	const manifest = readJsonFile<AppEvalCaseManifest[]>(manifestPath)

	return manifest.map((testCase) => ({
		...testCase,
		initialAppFixturePath: testCase.initialAppFixturePath
			? join(repoRoot, testCase.initialAppFixturePath)
			: undefined
	}))
}
