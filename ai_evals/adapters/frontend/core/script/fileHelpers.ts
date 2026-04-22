import { mkdir, rm, writeFile } from 'fs/promises'
import { dirname, join } from 'path'
import type { ScriptLang } from '../../../../../frontend/src/lib/gen/types.gen'
import type { ScriptChatHelpers } from '../../../../../frontend/src/lib/components/copilot/chat/script/core'
import { buildScriptLintResult } from './preview'
import { registerBenchmarkWorkspace, unregisterBenchmarkWorkspace } from '../../mockBackend'

export interface ScriptEvalState {
	code: string
	lang: ScriptLang | 'bunnative'
	path: string
	args: Record<string, any>
}

export async function createScriptFileHelpers(
	initialScript: ScriptEvalState,
	workspaceRoot?: string
): Promise<{
	helpers: ScriptChatHelpers
	getScript: () => ScriptEvalState
	cleanup: () => Promise<void>
	workspaceDir: string | null
}> {
	let script = structuredClone(initialScript)
	const scriptFilePath = workspaceRoot ? join(workspaceRoot, script.path) : null

	async function persistScript(): Promise<void> {
		if (!scriptFilePath) {
			return
		}
		await mkdir(dirname(scriptFilePath), { recursive: true })
		await writeFile(scriptFilePath, script.code, 'utf8')
	}

	await persistScript()

	if (workspaceRoot) {
		registerBenchmarkWorkspace(workspaceRoot)
	}

	const applyCode: NonNullable<ScriptChatHelpers['applyCode']> = async (
		code,
		opts
	) => {
		if (opts?.mode === 'revert') {
			return
		}
		script = {
			...script,
			code
		}
		await persistScript()
	}

	const getLintErrors: NonNullable<ScriptChatHelpers['getLintErrors']> = () =>
		buildScriptLintResult(script.code, script.lang)

	const helpers: ScriptChatHelpers = {
		getScriptOptions: () => ({
			code: script.code,
			lang: script.lang,
			path: script.path,
			args: structuredClone(script.args)
		}),
		applyCode,
		getLintErrors
	}

	return {
		helpers,
		getScript: () => structuredClone(script),
		cleanup: async () => {
			if (workspaceRoot) {
				unregisterBenchmarkWorkspace(workspaceRoot)
				await rm(workspaceRoot, { recursive: true, force: true })
			}
		},
		workspaceDir: workspaceRoot ?? null
	}
}
