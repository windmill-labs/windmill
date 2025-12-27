import processStdContent from '$lib/process.d.ts.txt?raw'
import { jsonDefaults } from '@codingame/monaco-vscode-standalone-json-language-features'
import {
	javascriptDefaults,
	JsxEmit,
	ModuleResolutionKind,
	ScriptTarget,
	typescriptDefaults
} from '@codingame/monaco-vscode-standalone-typescript-language-features'

let jsonInitialized = false
export function setMonacoJsonOptions() {
	if (jsonInitialized) return
	jsonInitialized = true
	jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})
	jsonDefaults.setModeConfiguration({
		documentRangeFormattingEdits: false,
		documentFormattingEdits: true,
		hovers: true,
		completionItems: true,
		documentSymbols: true,
		tokens: true,
		colors: true,
		foldingRanges: true,
		selectionRanges: true,
		diagnostics: true
	})
}

let typescriptInitialized = false

export function setMonacoTypescriptOptions() {
	if (typescriptInitialized) return
	typescriptInitialized = true

	typescriptDefaults.addExtraLib(processStdContent, 'process.d.ts')

	typescriptDefaults.setModeConfiguration({
		completionItems: true,
		hovers: true,
		documentSymbols: true,
		definitions: true,
		references: true,
		documentHighlights: true,
		rename: true,
		diagnostics: true,
		documentRangeFormattingEdits: true,
		signatureHelp: true,
		onTypeFormattingEdits: true,
		codeActions: true,
		inlayHints: true
	})

	// languages.typescript.javascriptDefaults.setEagerModelSync(true)
	typescriptDefaults.setEagerModelSync(true)

	// languages.typescript.javascriptDefaults.setDiagnosticsOptions({
	// 	noSemanticValidation: false,
	// 	noSyntaxValidation: false,
	// 	noSuggestionDiagnostics: false,
	// 	diagnosticCodesToIgnore: [1108]
	// })

	typescriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: false,
		noSyntaxValidation: false,

		noSuggestionDiagnostics: false,
		diagnosticCodesToIgnore: [1108, 7006, 7034, 7019, 7005]
	})

	typescriptDefaults.setCompilerOptions({
		target: ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noSemanticValidation: false,
		noSyntaxValidation: false,
		completionItems: true,
		hovers: true,
		documentSymbols: true,
		definitions: true,
		references: true,
		documentHighlights: true,
		rename: true,
		diagnostics: true,
		documentRangeFormattingEdits: true,
		signatureHelp: true,
		onTypeFormattingEdits: true,
		codeActions: true,
		inlayHints: true,
		checkJs: true,
		allowJs: true,
		noUnusedLocals: true,
		strict: true,
		noLib: false,
		allowImportingTsExtensions: true,
		allowSyntheticDefaultImports: true,
		moduleResolution: ModuleResolutionKind.NodeJs,
		jsx: JsxEmit.React
	})
}

let javascriptInitialized = false
export function setMonacoJavascriptOptions() {
	if (javascriptInitialized) return
	javascriptInitialized = true

	javascriptDefaults.setCompilerOptions({
		target: ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noSemanticValidation: false,
		noLib: true,
		moduleResolution: ModuleResolutionKind.NodeJs
	})
	javascriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: false,
		noSyntaxValidation: false,
		noSuggestionDiagnostics: false,
		diagnosticCodesToIgnore: [1108]
	})
}
