import ts from 'typescript'
import type { ScriptLang } from '../../../../../frontend/src/lib/gen/types.gen'
import type { ScriptLintResult } from '../../../../../frontend/src/lib/components/copilot/chat/shared'

export type ScriptPreviewLanguage = ScriptLang | 'bunnative'

const TS_LIKE_LANGUAGES = new Set<ScriptPreviewLanguage>(['bun', 'deno', 'nativets', 'bunnative'])
const JS_LIKE_LANGUAGES = new Set<ScriptPreviewLanguage>(['bun', 'deno', 'nativets', 'bunnative'])

function hasSupportedEntrypoint(code: string): boolean {
	return (
		/export\s+(async\s+)?function\s+main\s*\(/.test(code) ||
		/export\s+(async\s+)?function\s+preprocessor\s*\(/.test(code)
	)
}

function compilerOptionsForLanguage(lang: ScriptPreviewLanguage): ts.CompilerOptions | null {
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

export function buildScriptLintResult(
	code: string,
	lang: ScriptPreviewLanguage
): ScriptLintResult {
	const diagnostics: ScriptLintResult['errors'] = []
	const compilerOptions = compilerOptionsForLanguage(lang)

	if (compilerOptions) {
		const sourceFile = ts.createSourceFile(
			'script.ts',
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
