// @ts-ignore - Node.js fs
import { existsSync, readFileSync } from 'fs'
// @ts-ignore - Node.js path
import { dirname, join, resolve } from 'path'
// @ts-ignore - Node.js url
import { fileURLToPath } from 'url'
import type { ScriptLang } from '$lib/gen/types.gen'

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

export interface ScriptEvalFixture {
	code: string
	lang: ScriptLang | 'bunnative'
	path: string
	args?: Record<string, any>
}

export interface ScriptEvalCaseManifest {
	id: string
	title: string
	userPrompt: string
	expectedScriptPath: string
	initialScriptPath?: string
	minJudgeScore?: number
}

export interface ScriptEvalCase extends ScriptEvalCaseManifest {
	expectedScript: ScriptEvalFixture
	initialScript?: ScriptEvalFixture
}

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

export function loadScriptEvalCases(): ScriptEvalCase[] {
	const repoRoot = resolveRepoRoot()
	const manifestPath = join(repoRoot, 'ai_evals', 'cases', 'frontend', 'script.json')
	const manifest = readJsonFile<ScriptEvalCaseManifest[]>(manifestPath)

	return manifest.map((testCase) => ({
		...testCase,
		expectedScript: readJsonFile<ScriptEvalFixture>(join(repoRoot, testCase.expectedScriptPath)),
		initialScript: testCase.initialScriptPath
			? readJsonFile<ScriptEvalFixture>(join(repoRoot, testCase.initialScriptPath))
			: undefined
	}))
}
