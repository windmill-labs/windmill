import { languages } from 'monaco-editor/esm/vs/editor/editor.api'

import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'

export function editorConfig(
	code: string,
	lang: string,
	automaticLayout: boolean,
	fixedOverflowWidgets: boolean
) {
	return {
		value: code,
		language: lang,
		automaticLayout,
		readOnly: false,
		fixedOverflowWidgets,
		lineDecorationsWidth: 10,
		lineNumbersMinChars: 3,
		scrollbar: { alwaysConsumeMouseWheel: false },
		folding: false,
		scrollBeyondLastLine: false,
		glyphMargin: false,
		minimap: {
			enabled: false
		},
		lightbulb: {
			enabled: meditor.ShowLightbulbIconMode.On
		},
		suggest: {
			showKeywords: true
		},
		bracketPairColorization: {
			enabled: true
		},
		'workbench.colorTheme': 'Default Dark Modern',
		workbench: {
			colorTheme: 'Default Dark Modern'
		},
		'bracketPairColorization.enabled': true,
		matchBrackets: 'always' as 'always'
	}
}

export function createHash() {
	return (Math.random() + 1).toString(36).substring(2)
}

export function createLongHash() {
	return 'h' + Math.random().toString(36).substring(2) + Math.random().toString(36).substring(2)
}

export function langToExt(lang: string): string {
	switch (lang) {
		case 'tsx':
			return 'tsx'
		case 'javascript':
			return 'js'
		case 'jsx':
			return 'js'
		case 'bunnative':
			return 'ts'
		case 'json':
			return 'json'
		case 'sql':
			return 'sql'
		case 'yaml':
			return 'yaml'
		case 'typescript':
			return 'ts'
		case 'python':
			return 'py'
		case 'go':
			return 'go'
		case 'bash':
			return 'sh'
		case 'powershell':
			return 'ps1'
		case 'php':
			return 'php'
		case 'rust':
			return 'rs'
		case 'deno':
			return 'ts'
		case 'nativets':
			return 'ts'
		case 'graphql':
			return 'gql'
		case 'css':
			return 'css'
		case 'ansible':
			return 'yml'
		case 'csharp':
			return 'cs'
		case 'nu':
			return 'nu'
		case 'java':
			return 'java'
		case 'ruby':
			return 'rb'
		// for related places search: ADD_NEW_LANG 
		case 'svelte':
			return 'svelte'
		case 'vue':
			return 'vue'
		default:
			return 'unknown'
	}
}

export function extToLang(ext: string) {
	switch (ext) {
		case 'tsx':
			return 'typescript'
		case 'ts':
			return 'typescript'
		case 'js':
			return 'javascript'
		case 'jsx':
			return 'javascript'
		case 'json':
			return 'json'
		case 'sql':
			return 'sql'
		case 'yaml':
			return 'yaml'
		case 'py':
			return 'python'
		case 'go':
			return 'go'
		case 'sh':
			return 'bash'
		case 'ps1':
			return 'powershell'
		case 'php':
			return 'php'
		case 'rs':
			return 'rust'
		case 'gql':
			return 'graphql'
		case 'css':
			return 'css'
		case 'yml':
			return 'ansible'
		case 'cs':
			return 'csharp'
		case 'svelte':
			return 'svelte'
		case 'vue':
			return 'vue'
		case 'nu':
			return 'nu'
		case 'java':
			return 'java'
		// for related places search: ADD_NEW_LANG
		default:
			return 'unknown'
	}
}

export const updateOptions = { tabSize: 2, insertSpaces: true }

export function convertKind(kind: string): any {
	switch (kind) {
		case Kind.primitiveType:
		case Kind.keyword:
			return languages.CompletionItemKind.Keyword
		case Kind.variable:
		case Kind.localVariable:
			return languages.CompletionItemKind.Variable
		case Kind.memberVariable:
		case Kind.memberGetAccessor:
		case Kind.memberSetAccessor:
			return languages.CompletionItemKind.Field
		case Kind.function:
		case Kind.memberFunction:
		case Kind.constructSignature:
		case Kind.callSignature:
		case Kind.indexSignature:
			return languages.CompletionItemKind.Function
		case Kind.enum:
			return languages.CompletionItemKind.Enum
		case Kind.module:
			return languages.CompletionItemKind.Module
		case Kind.class:
			return languages.CompletionItemKind.Class
		case Kind.interface:
			return languages.CompletionItemKind.Interface
		case Kind.warning:
			return languages.CompletionItemKind.File
	}

	return languages.CompletionItemKind.Property
}

class Kind {
	public static unknown: string = ''
	public static keyword: string = 'keyword'
	public static script: string = 'script'
	public static module: string = 'module'
	public static class: string = 'class'
	public static interface: string = 'interface'
	public static type: string = 'type'
	public static enum: string = 'enum'
	public static variable: string = 'var'
	public static localVariable: string = 'local var'
	public static function: string = 'function'
	public static localFunction: string = 'local function'
	public static memberFunction: string = 'method'
	public static memberGetAccessor: string = 'getter'
	public static memberSetAccessor: string = 'setter'
	public static memberVariable: string = 'property'
	public static constructorImplementation: string = 'constructor'
	public static callSignature: string = 'call'
	public static indexSignature: string = 'index'
	public static constructSignature: string = 'construct'
	public static parameter: string = 'parameter'
	public static typeParameter: string = 'type parameter'
	public static primitiveType: string = 'primitive type'
	public static label: string = 'label'
	public static alias: string = 'alias'
	public static const: string = 'const'
	public static let: string = 'let'
	public static warning: string = 'warning'
}

export function createDocumentationString(details: any): string {
	let documentationString = displayPartsToString(details.documentation)
	if (details.tags) {
		for (const tag of details.tags) {
			documentationString += `\n\n${tagToString(tag)}`
		}
	}
	return documentationString
}

function tagToString(tag: any): string {
	let tagLabel = `*@${tag.name}*`
	if (tag.name === 'param' && tag.text) {
		const [paramName, ...rest] = tag.text
		tagLabel += `\`${paramName.text}\``
		if (rest.length > 0) tagLabel += ` — ${rest.map((r) => r.text).join(' ')}`
	} else if (Array.isArray(tag.text)) {
		tagLabel += ` — ${tag.text.map((r) => r.text).join(' ')}`
	} else if (tag.text) {
		tagLabel += ` — ${tag.text}`
	}
	return tagLabel
}

export function displayPartsToString(displayParts: any | undefined): string {
	if (displayParts) {
		return displayParts.map((displayPart) => displayPart.text).join('')
	}
	return ''
}
