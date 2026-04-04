import { mkdir, rm, writeFile } from 'fs/promises'
import { dirname, join } from 'path'
import ts from 'typescript'
import type { ScriptLang } from '../../../../../frontend/src/lib/gen/types.gen'
import type { ReviewChangesOpts } from '../../../../../frontend/src/lib/components/copilot/chat/monaco-adapter'
import type { ScriptLintResult } from '../../../../../frontend/src/lib/components/copilot/chat/shared'
import type { ScriptChatHelpers } from '../../../../../frontend/src/lib/components/copilot/chat/script/core'

export interface ScriptEvalState {
	code: string
	lang: ScriptLang | 'bunnative'
	path: string
	args: Record<string, any>
}

const TS_LIKE_LANGUAGES = new Set<ScriptLang | 'bunnative'>(['bun', 'deno', 'nativets', 'bunnative'])
const JS_LIKE_LANGUAGES = new Set<ScriptLang | 'bunnative'>(['bun', 'deno', 'nativets', 'bunnative'])

function hasSupportedEntrypoint(code: string): boolean {
	return (
		/export\s+(async\s+)?function\s+main\s*\(/.test(code) ||
		/export\s+(async\s+)?function\s+preprocessor\s*\(/.test(code)
	)
}

function compilerOptionsForLanguage(lang: ScriptLang | 'bunnative'): ts.CompilerOptions | null {
	if (!TS_LIKE_LANGUAGES.has(lang)) {
		return null
	}

	return {
		target: ts.ScriptTarget.ES2022,
		module: ts.ModuleKind.ESNext,
		moduleResolution: ts.ModuleResolutionKind.Bundler,
		noEmit: true,
		allowJs: true,
		checkJs: false,
		strict: false,
		skipLibCheck: true
	}
}

function getLineAndColumn(sourceText: string, start: number): { line: number; column: number } {
	const prefix = sourceText.slice(0, Math.max(0, start))
	const line = prefix.split('\n').length
	const lastNewline = prefix.lastIndexOf('\n')
	const column = lastNewline === -1 ? prefix.length + 1 : prefix.length - lastNewline
	return { line, column }
}

function buildLintResult(code: string, lang: ScriptLang | 'bunnative'): ScriptLintResult {
	const diagnostics: ScriptLintResult['errors'] = []
	const compilerOptions = compilerOptionsForLanguage(lang)

	if (compilerOptions) {
		const sourceFile = ts.createSourceFile(
			lang === 'deno' ? 'script.ts' : 'script.ts',
			code,
			ts.ScriptTarget.ES2022,
			true,
			JS_LIKE_LANGUAGES.has(lang) ? ts.ScriptKind.TS : ts.ScriptKind.JS
		)
		const output = ts.transpileModule(code, {
			compilerOptions,
			fileName: sourceFile.fileName,
			reportDiagnostics: true
		})

		for (const diagnostic of output.diagnostics ?? []) {
			const start = diagnostic.start ?? 0
			const length = diagnostic.length ?? 1
			const { line, column } = getLineAndColumn(code, start)
			const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n')
			diagnostics.push({
				startLineNumber: line,
				startColumn: column,
				endLineNumber: line,
				endColumn: column + Math.max(1, length),
				message,
				severity: 8
			} as ScriptLintResult['errors'][number])
		}
	}

	if (!hasSupportedEntrypoint(code)) {
		diagnostics.push({
			startLineNumber: 1,
			startColumn: 1,
			endLineNumber: 1,
			endColumn: 1,
			message: 'Script must export a main or preprocessor function.',
			severity: 8
		} as ScriptLintResult['errors'][number])
	}

	return {
		errorCount: diagnostics.length,
		warningCount: 0,
		errors: diagnostics,
		warnings: []
	}
}

function formatScriptTestSummary(success: boolean, result: Record<string, unknown>): string {
	return `Result (${success ? 'SUCCESS' : 'FAILED'})\n\n${JSON.stringify(result, null, 2)}\n\nLogs:\n\nNo logs available`
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

	const helpers: ScriptChatHelpers = {
		getScriptOptions: () => ({
			code: script.code,
			lang: script.lang,
			path: script.path,
			args: structuredClone(script.args)
		}),
		applyCode: async (code: string, opts?: ReviewChangesOpts) => {
			if (opts?.mode === 'revert') {
				return
			}
			script = {
				...script,
				code
			}
			await persistScript()
		},
		getLintErrors: () => buildLintResult(script.code, script.lang),
		runScriptTest: async (args: Record<string, any>) => {
			const lintResult = buildLintResult(script.code, script.lang)
			const success = lintResult.errorCount === 0

			if (workspaceRoot) {
				await writeFile(
					join(workspaceRoot, 'test-run.json'),
					JSON.stringify(
						{
							requestedArgs: args,
							success,
							errorCount: lintResult.errorCount,
							path: script.path
						},
						null,
						2
					) + '\n',
					'utf8'
				)
			}

			return formatScriptTestSummary(
				success,
				success
					? {
							path: script.path,
							args,
							validated: true
						}
					: {
							path: script.path,
							args,
							errorCount: lintResult.errorCount,
							errors: lintResult.errors.map((entry) => ({
								line: entry.startLineNumber,
								message: entry.message
							}))
						}
			)
		}
	}

	return {
		helpers,
		getScript: () => structuredClone(script),
		cleanup: async () => {
			if (workspaceRoot) {
				await rm(workspaceRoot, { recursive: true, force: true })
			}
		},
		workspaceDir: workspaceRoot ?? null
	}
}
