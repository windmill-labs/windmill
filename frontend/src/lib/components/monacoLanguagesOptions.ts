import { languages } from 'monaco-editor'
import processStdContent from '$lib/process.d.ts.txt?raw'

let jsonInitialized = false
export function setMonacoJsonOptions() {
	if (jsonInitialized) return
	jsonInitialized = true
	languages.json.jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})
	languages.json.jsonDefaults.setModeConfiguration({
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

	languages.typescript.typescriptDefaults.addExtraLib(processStdContent, 'process.d.ts')

	languages.typescript.typescriptDefaults.setModeConfiguration({
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
	languages.typescript.typescriptDefaults.setEagerModelSync(true)

	// languages.typescript.javascriptDefaults.setDiagnosticsOptions({
	// 	noSemanticValidation: false,
	// 	noSyntaxValidation: false,
	// 	noSuggestionDiagnostics: false,
	// 	diagnosticCodesToIgnore: [1108]
	// })

	languages.typescript.typescriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: false,
		noSyntaxValidation: false,

		noSuggestionDiagnostics: false,
		diagnosticCodesToIgnore: [1108, 7006, 7034, 7019, 7005]
	})

	languages.typescript.typescriptDefaults.setCompilerOptions({
		target: languages.typescript.ScriptTarget.Latest,
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
		moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs,
		jsx: languages.typescript.JsxEmit.React
	})
}

let javascriptInitialized = false
export function setMonacoJavascriptOptions() {
	if (javascriptInitialized) return
	javascriptInitialized = true

	languages.typescript.javascriptDefaults.setCompilerOptions({
		target: languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noSemanticValidation: false,
		noLib: true,
		moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs
	})
	languages.typescript.javascriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: false,
		noSyntaxValidation: false,
		noSuggestionDiagnostics: false,
		diagnosticCodesToIgnore: [1108]
	})
}
