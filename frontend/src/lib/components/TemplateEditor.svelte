<script lang="ts" context="module">
	import * as monaco from 'monaco-editor'
	import libStdContent from '$lib/es5.d.ts.txt?raw'

	monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
		target: monaco.languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noLib: true
	})

	monaco.languages.register({ id: 'template' })

	export const conf = {
		wordPattern:
			/(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,

		comments: {
			lineComment: '//',
			blockComment: ['/*', '*/'] as [string, string]
		},

		brackets: [
			['{', '}'],
			['[', ']'],
			['(', ')']
		] as [string, string][],

		onEnterRules: [],
		autoClosingPairs: [
			{ open: '{', close: '}' },
			{ open: '[', close: ']' },
			{ open: '(', close: ')' },
			{ open: '"', close: '"', notIn: ['string'] },
			{ open: "'", close: "'", notIn: ['string', 'comment'] },
			{ open: '`', close: '`', notIn: ['string', 'comment'] }
		],

		folding: {
			markers: {
				start: new RegExp('^\\s*//\\s*#?region\\b'),
				end: new RegExp('^\\s*//\\s*#?endregion\\b')
			}
		}
	}

	export const language = {
		// Set defaultToken to invalid to see what you do not tokenize yet
		defaultToken: 'invalid',
		tokenPostfix: '.ts',

		keywords: [
			// Should match the keys of textToKeywordObj in
			// https://github.com/microsoft/TypeScript/blob/master/src/compiler/scanner.ts
			'abstract',
			'any',
			'as',
			'asserts',
			'bigint',
			'boolean',
			'break',
			'case',
			'catch',
			'class',
			'continue',
			'const',
			'constructor',
			'debugger',
			'declare',
			'default',
			'delete',
			'do',
			'else',
			'enum',
			'export',
			'extends',
			'false',
			'finally',
			'for',
			'from',
			'function',
			'get',
			'if',
			'implements',
			'import',
			'in',
			'infer',
			'instanceof',
			'interface',
			'is',
			'keyof',
			'let',
			'module',
			'namespace',
			'never',
			'new',
			'null',
			'number',
			'object',
			'out',
			'package',
			'private',
			'protected',
			'public',
			'override',
			'readonly',
			'require',
			'global',
			'return',
			'set',
			'static',
			'string',
			'super',
			'switch',
			'symbol',
			'this',
			'throw',
			'true',
			'try',
			'type',
			'typeof',
			'undefined',
			'unique',
			'unknown',
			'var',
			'void',
			'while',
			'with',
			'yield',
			'async',
			'await',
			'of'
		],

		operators: [
			'<=',
			'>=',
			'==',
			'!=',
			'===',
			'!==',
			'=>',
			'+',
			'-',
			'**',
			'*',
			'/',
			'%',
			'++',
			'--',
			'<<',
			'</',
			'>>',
			'>>>',
			'&',
			'|',
			'^',
			'!',
			'~',
			'&&',
			'||',
			'??',
			'?',
			':',
			'=',
			'+=',
			'-=',
			'*=',
			'**=',
			'/=',
			'%=',
			'<<=',
			'>>=',
			'>>>=',
			'&=',
			'|=',
			'^=',
			'@'
		],

		// we include these common regular expressions
		symbols: /[=><!~?:&|+\-*\/\^%]+/,
		escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,
		digits: /\d+(_+\d+)*/,
		octaldigits: /[0-7]+(_+[0-7]+)*/,
		binarydigits: /[0-1]+(_+[0-1]+)*/,
		hexdigits: /[[0-9a-fA-F]+(_+[0-9a-fA-F]+)*/,

		regexpctl: /[(){}\[\]\$\^|\-*+?\.]/,
		regexpesc: /\\(?:[bBdDfnrstvwWn0\\\/]|@regexpctl|c[A-Z]|x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4})/,

		// The main tokenizer for our languages
		tokenizer: {
			root: [
				[/\$\{/, { token: 'delimiter.bracket', next: '@bracketCounting' }],
				[/[^\\`$]+/, 'string'],
				[/@escapes/, 'string.escape'],
				[/\\./, 'string.escape.invalid']
			],

			common: [
				// identifiers and keywords
				[
					/[a-z_$][\w$]*/,
					{
						cases: {
							'@keywords': 'keyword',
							'@default': 'identifier'
						}
					}
				],
				[/[A-Z][\w\$]*/, 'type.identifier'], // to show class names nicely
				// [/[A-Z][\w\$]*/, 'identifier'],

				// whitespace
				{ include: '@whitespace' },

				// regular expression: ensure it is terminated before beginning (otherwise it is an opeator)
				[
					/\/(?=([^\\\/]|\\.)+\/([dgimsuy]*)(\s*)(\.|;|,|\)|\]|\}|$))/,
					{ token: 'regexp', bracket: '@open', next: '@regexp' }
				],

				// delimiters and operators
				[/[()\[\]]/, '@brackets'],
				[/[<>](?!@symbols)/, '@brackets'],
				[/!(?=([^=]|$))/, 'delimiter'],
				[
					/@symbols/,
					{
						cases: {
							'@operators': 'delimiter',
							'@default': ''
						}
					}
				],

				// numbers
				[/(@digits)[eE]([\-+]?(@digits))?/, 'number.float'],
				[/(@digits)\.(@digits)([eE][\-+]?(@digits))?/, 'number.float'],
				[/0[xX](@hexdigits)n?/, 'number.hex'],
				[/0[oO]?(@octaldigits)n?/, 'number.octal'],
				[/0[bB](@binarydigits)n?/, 'number.binary'],
				[/(@digits)n?/, 'number'],

				// delimiter: after number because of .\d floats
				[/[;,.]/, 'delimiter'],

				// strings
				[/"([^"\\]|\\.)*$/, 'string.invalid'], // non-teminated string
				[/'([^'\\]|\\.)*$/, 'string.invalid'], // non-teminated string
				[/"/, 'string', '@string_double'],
				[/'/, 'string', '@string_single'],
				[/`/, 'string', '@string_backtick']
			],

			whitespace: [
				[/[ \t\r\n]+/, ''],
				[/\/\*\*(?!\/)/, 'comment.doc', '@jsdoc'],
				[/\/\*/, 'comment', '@comment'],
				[/\/\/.*$/, 'comment']
			],

			comment: [
				[/[^\/*]+/, 'comment'],
				[/\*\//, 'comment', '@pop'],
				[/[\/*]/, 'comment']
			],

			jsdoc: [
				[/[^\/*]+/, 'comment.doc'],
				[/\*\//, 'comment.doc', '@pop'],
				[/[\/*]/, 'comment.doc']
			],

			// We match regular expression quite precisely
			regexp: [
				[
					/(\{)(\d+(?:,\d*)?)(\})/,
					['regexp.escape.control', 'regexp.escape.control', 'regexp.escape.control']
				],
				[
					/(\[)(\^?)(?=(?:[^\]\\\/]|\\.)+)/,
					['regexp.escape.control', { token: 'regexp.escape.control', next: '@regexrange' }]
				],
				[/(\()(\?:|\?=|\?!)/, ['regexp.escape.control', 'regexp.escape.control']],
				[/[()]/, 'regexp.escape.control'],
				[/@regexpctl/, 'regexp.escape.control'],
				[/[^\\\/]/, 'regexp'],
				[/@regexpesc/, 'regexp.escape'],
				[/\\\./, 'regexp.invalid'],
				[
					/(\/)([dgimsuy]*)/,
					[{ token: 'regexp', bracket: '@close', next: '@pop' }, 'keyword.other']
				]
			],

			regexrange: [
				[/-/, 'regexp.escape.control'],
				[/\^/, 'regexp.invalid'],
				[/@regexpesc/, 'regexp.escape'],
				[/[^\]]/, 'regexp'],
				[
					/\]/,
					{
						token: 'regexp.escape.control',
						next: '@pop',
						bracket: '@close'
					}
				]
			],

			string_double: [
				[/[^\\"]+/, 'string'],
				[/@escapes/, 'string.escape'],
				[/\\./, 'string.escape.invalid'],
				[/"/, 'string', '@pop']
			],

			string_single: [
				[/[^\\']+/, 'string'],
				[/@escapes/, 'string.escape'],
				[/\\./, 'string.escape.invalid'],
				[/'/, 'string', '@pop']
			],

			string_backtick: [
				[/\$\{/, { token: 'delimiter.bracket', next: '@bracketCounting' }],
				[/[^\\`$]+/, 'string'],
				[/@escapes/, 'string.escape'],
				[/\\./, 'string.escape.invalid'],
				[/`/, 'string', '@pop']
			],

			bracketCounting: [
				[/\{/, 'delimiter.bracket', '@bracketCounting'],
				[/\}/, 'delimiter.bracket', '@pop'],
				{ include: 'common' }
			]
		}
	}

	// Register a tokens provider for the language
	monaco.languages.registerTokensProviderFactory('template', {
		create: () => language as monaco.languages.IMonarchLanguage
	})

	monaco.languages.setLanguageConfiguration('template', conf)

	// monaco.languages.typescript.getTypeScriptWorker()

	// Register a completion item provider for the new language
</script>

<script lang="ts">
	import { browser, dev } from '$app/env'

	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	import { buildWorkerDefinition } from 'monaco-editor-workers'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import {
		convertKind,
		createDocumentationString,
		createHash,
		displayPartsToString,
		editorConfig,
		updateOptions
	} from '$lib/editorUtils'

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor
	let model: monaco.editor.ITextModel

	export let code: string = ''
	export let hash: string = createHash()
	export let automaticLayout = true
	export let extraLib: string = ''
	export let autoHeight = true
	export let fixedOverflowWidgets = true

	const lang = 'template'
	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.ts`

	if (browser) {
		if (dev) {
			buildWorkerDefinition(
				'../../../node_modules/monaco-editor-workers/dist/workers',
				import.meta.url,
				false
			)
		} else {
			// @ts-ignore
			self.MonacoEnvironment = {
				getWorker: function (_moduleId: any, label: string) {
					return new tsWorker()
				}
			}
		}
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
		}
	}

	export function setCode(ncode: string): void {
		code = ncode
		if (editor) {
			editor.setValue(ncode)
		}
	}

	export function getCode(): string {
		return editor?.getValue() ?? ''
	}

	let cip
	let extraModel

	let width = 0
	async function loadMonaco() {
		model = monaco.editor.createModel(code, lang, monaco.Uri.parse(uri))

		model.updateOptions(updateOptions)

		editor = monaco.editor.create(divEl as HTMLDivElement, {
			...editorConfig(model, code, lang, automaticLayout, fixedOverflowWidgets),
			lineNumbers: 'off',
			fontSize: 16,
			suggestOnTriggerCharacters: true,
			lineDecorationsWidth: 0
		})

		const stdLib = { content: libStdContent, filePath: 'es5.d.ts' }
		if (extraLib != '') {
			monaco.languages.typescript.javascriptDefaults.setExtraLibs([
				{
					content: extraLib,
					filePath: 'windmill.d.ts'
				},
				stdLib
			])
		} else {
			monaco.languages.typescript.javascriptDefaults.setExtraLibs([stdLib])
		}

		extraModel = monaco.editor.createModel('`' + model.getValue() + '`', 'javascript')
		const worker = await monaco.languages.typescript.getJavaScriptWorker()
		const client = await worker(extraModel.uri)

		cip = monaco.languages.registerCompletionItemProvider('template', {
			triggerCharacters: ['.'],

			provideCompletionItems: async (model, position) => {
				extraModel.setValue('`' + model.getValue() + '`')

				const offset = model.getOffsetAt(position) + 1
				const info = await client.getCompletionsAtPosition(extraModel.uri.toString(), offset)
				if (!info) {
					return { suggestions: [] }
				}
				const wordInfo = model.getWordUntilPosition(position)
				const wordRange = new monaco.Range(
					position.lineNumber,
					wordInfo.startColumn,
					position.lineNumber,
					wordInfo.endColumn
				)

				const suggestions = info.entries
					.filter((x) => x.kind != 'keyword' && x.kind != 'var')
					.map((entry) => {
						let range = wordRange
						if (entry.replacementSpan) {
							const p1 = model.getPositionAt(entry.replacementSpan.start)
							const p2 = model.getPositionAt(
								entry.replacementSpan.start + entry.replacementSpan.length
							)
							range = new monaco.Range(p1.lineNumber, p1.column, p2.lineNumber, p2.column)
						}

						const tags: monaco.languages.CompletionItemTag[] = []
						if (entry.kindModifiers?.indexOf('deprecated') !== -1) {
							tags.push(monaco.languages.CompletionItemTag.Deprecated)
						}
						return {
							uri: model.uri,
							position: position,
							offset: offset,
							range: range,
							label: entry.name,
							insertText: entry.name,
							sortText: entry.sortText,
							kind: convertKind(entry.kind),
							tags
						}
					})
				return { suggestions }
			},
			resolveCompletionItem: async (item: monaco.languages.CompletionItem, token: any) => {
				extraModel.setValue('`' + model.getValue() + '`')

				const myItem = <any>item
				const position = myItem.position
				const offset = myItem.offset

				const details = await client.getCompletionEntryDetails(
					extraModel.uri.toString(),
					offset,
					myItem.label
				)
				if (!details) {
					return myItem
				}
				return <any>{
					uri: model.uri,
					position: position,
					label: details.name,
					kind: convertKind(details.kind),
					detail: displayPartsToString(details.displayParts),
					documentation: {
						value: createDocumentationString(details)
					}
				}
			}
		})

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
				dispatch('change', { code })
			}, 200)
		})

		if (autoHeight) {
			let ignoreEvent = false
			const updateHeight = () => {
				const contentHeight = Math.min(1000, editor.getContentHeight())
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					ignoreEvent = true
					editor.layout({ width, height: contentHeight })
				} finally {
					ignoreEvent = false
				}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			code = getCode()
			dispatch('blur')
		})
	}

	onMount(() => {
		if (browser) {
			loadMonaco()
		}
	})

	onDestroy(() => {
		try {
			model && model.dispose()
			editor && editor.dispose()
			cip && cip.dispose()
			extraModel && extraModel.dispose()
		} catch (err) {}
	})
</script>

<div bind:this={divEl} class="{$$props.class} editor templatable-editor" bind:clientWidth={width} />

<style>
	.editor {
		@apply rounded-lg mx-0.5;
	}
</style>
