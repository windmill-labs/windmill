<script module>
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import {
		javascriptDefaults,
		getJavaScriptWorker
	} from '@codingame/monaco-vscode-standalone-typescript-language-features'
</script>

<script lang="ts">
	import { BROWSER } from 'esm-env'
	import {
		convertKind,
		createDocumentationString,
		displayPartsToString,
		editorConfig,
		updateOptions
	} from '$lib/editorUtils'
	import { createHash } from '$lib/editorLangUtils'

	import libStdContent from '$lib/es6.d.ts.txt?raw'
	import { editor as meditor, Uri as mUri, languages, Range, KeyMod, KeyCode } from 'monaco-editor'
	import { createEventDispatcher, getContext, onDestroy, onMount, untrack } from 'svelte'
	import type { AppViewerContext } from './apps/types'
	import { writable } from 'svelte/store'
	// import '@codingame/monaco-vscode-standalone-languages'

	// import '@codingame/monaco-vscode-standalone-typescript-language-features'

	import { initializeVscode, MONACO_Y_PADDING } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import FakeMonacoPlaceHolder from './FakeMonacoPlaceHolder.svelte'
	import { setMonacoJsonOptions } from './monacoLanguagesOptions'
	import { inputBorderClass } from './text_input/TextInput.svelte'
	import { twMerge } from 'tailwind-merge'

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

	let divEl: HTMLDivElement | null = $state(null)
	let editor: meditor.IStandaloneCodeEditor | undefined = $state(undefined)
	let model: meditor.ITextModel

	const { componentControl, selectedComponent } = getContext<AppViewerContext>(
		'AppViewerContext'
	) || { componentControl: writable({}), selectedComponent: writable([]) }

	if ($selectedComponent) {
		$componentControl[$selectedComponent[0]] = {
			...$componentControl[$selectedComponent[0]],
			setCode: (value: string) => {
				setCode(value)
			}
		}
	}

	interface Props {
		code?: string
		hash?: string
		automaticLayout?: boolean
		extraLib?: string
		autoHeight?: boolean
		fixedOverflowWidgets?: boolean
		fontSize?: number
		loadAsync?: boolean
		class?: string | undefined
	}

	let {
		code = $bindable(),
		hash = createHash(),
		automaticLayout = true,
		extraLib = '',
		autoHeight = true,
		fixedOverflowWidgets = true,
		fontSize = 12,
		loadAsync = false,
		class: clazz = ''
	}: Props = $props()

	let yPadding = MONACO_Y_PADDING

	if (typeof code != 'string') {
		code = ''
	}

	const lang = 'template'
	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.ts`

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
		}
	}

	export function setCode(ncode: string): void {
		if (code != ncode) {
			code = ncode
		}
		editor?.setValue(ncode)
	}

	let valueAfterDispose: string | undefined = undefined
	export function getCode(): string {
		if (valueAfterDispose != undefined) {
			return valueAfterDispose
		}
		return editor?.getValue() ?? ''
	}

	let cip
	let extraModel

	let width = $state(0)
	// let widgets: HTMLElement | undefined = document.getElementById('monaco-widgets-root') ?? undefined

	let initialized = $state(false)

	let jsLoader: number | undefined = undefined
	let timeoutModel: number | undefined = undefined
	async function loadMonaco() {
		setMonacoJsonOptions()
		await initializeVscode('templateEditor')
		initialized = true

		languages.register({ id: 'template' })

		// Register a tokens provider for the language
		languages.registerTokensProviderFactory('template', {
			create: () => language as languages.IMonarchLanguage
		})

		languages.setLanguageConfiguration('template', conf)

		model = meditor.createModel(code ?? '', lang, mUri.parse(uri))

		model.updateOptions(updateOptions)

		try {
			editor = meditor.create(divEl as HTMLDivElement, {
				...editorConfig(code ?? '', lang, automaticLayout, fixedOverflowWidgets, false),
				model,
				// overflowWidgetsDomNode: widgets,
				// lineNumbers: 'on',
				lineDecorationsWidth: 0,
				lineNumbersMinChars: 2,
				fontSize,
				suggestOnTriggerCharacters: true,
				renderLineHighlight: 'none',
				lineNumbers: 'off',

				...(yPadding !== undefined ? { padding: { bottom: yPadding, top: yPadding } } : {})
			})
		} catch (e) {
			console.error('Error loading monaco:', e)
			return
		}

		// In VSCode webview (iframe), clipboard operations need special handling
		// because the webview has restricted clipboard API access
		if (window.parent !== window) {
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyC, function () {
				document.execCommand('copy')
			})
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyX, function () {
				document.execCommand('cut')
			})
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyV, async function () {
				try {
					const text = await navigator.clipboard.readText()
					if (text && editor) {
						const selection = editor.getSelection()
						if (selection) {
							editor.executeEdits('paste', [
								{
									range: selection,
									text: text,
									forceMoveMarkers: true
								}
							])
						}
					}
				} catch (e) {
					document.execCommand('paste')
				}
			})
		}

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {})

			editor?.addCommand(KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.Digit7, function () {})
		})

		function updateCode() {
			const ncode = getCode()
			if (code == ncode) {
				return
			}
			code = ncode
			dispatch('change', { code: ncode })
		}

		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				updateCode()
			}, 200)
		})

		extraModel = meditor.createModel('`' + model.getValue() + '`', 'javascript')

		if (autoHeight) {
			const updateHeight = () => {
				const contentHeight = Math.min(1000, editor?.getContentHeight() ?? 0)
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					editor?.layout({ width, height: contentHeight })
				} catch {}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			dispatch('focus')
			isFocus = true
		})

		editor.onDidBlurEditorText(() => {
			dispatch('blur')
			isFocus = false
			updateCode()
		})

		jsLoader = setTimeout(async () => {
			jsLoader = undefined
			try {
				const worker = await getJavaScriptWorker()
				const client = await worker(extraModel.uri)

				cip = languages.registerCompletionItemProvider('template', {
					triggerCharacters: ['.'],

					provideCompletionItems: async (model, position) => {
						extraModel.setValue('`' + model.getValue() + '`')

						const offset = model.getOffsetAt(position) + 1
						const info = await client.getCompletionsAtPosition(extraModel.uri.toString(), offset)
						if (!info) {
							return { suggestions: [] }
						}
						const wordInfo = model.getWordUntilPosition(position)
						const wordRange = new Range(
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
									range = new Range(p1.lineNumber, p1.column, p2.lineNumber, p2.column)
								}

								const tags: languages.CompletionItemTag[] = []
								if (entry.kindModifiers?.indexOf('deprecated') !== -1) {
									tags.push(languages.CompletionItemTag.Deprecated)
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
					resolveCompletionItem: async (item: languages.CompletionItem, token: any) => {
						extraModel.setValue('`' + model.getValue() + '`')

						const myItem = item as any
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
						return {
							uri: model.uri,
							position: position,
							label: details.name,
							kind: convertKind(details.kind),
							detail: displayPartsToString(details.displayParts),
							documentation: {
								value: createDocumentationString(details)
							}
						} as any
					}
				})
			} catch (e) {
				console.error('Error loading javascipt worker:', e)
			}
		}, 300)
	}

	export function focus() {
		editor?.focus()
	}

	let isFocus = $state(false)
	let mounted = $state(false)
	let loadTimeout: number | undefined = undefined
	onMount(async () => {
		try {
			if (BROWSER) {
				if (loadAsync) {
					loadTimeout = setTimeout(async () => {
						await loadMonaco()
						mounted = true
					}, 0)
				} else {
					await loadMonaco()
					mounted = true
				}
			}
		} catch (e) {
			console.error('Error loading monaco:', e)
		}
	})

	function loadExtraLib() {
		const stdLib = { content: libStdContent, filePath: 'es6.d.ts' }
		const libs = [stdLib]
		if (extraLib != '') {
			libs.push({
				content: extraLib,
				filePath: 'windmill.d.ts'
			})
		}
		javascriptDefaults.setExtraLibs(libs)
	}

	onDestroy(() => {
		try {
			valueAfterDispose = getCode()
			jsLoader && clearTimeout(jsLoader)
			timeoutModel && clearTimeout(timeoutModel)
			loadTimeout && clearTimeout(loadTimeout)
			model && model.dispose()
			editor && editor.dispose()
			cip && cip.dispose()
			extraModel && extraModel.dispose()
		} catch (err) {}
	})
	$effect(() => {
		mounted && extraLib && initialized && untrack(() => loadExtraLib())
	})
</script>

<EditorTheme />

<div
	class={twMerge(inputBorderClass({ forceFocus: isFocus }), 'rounded-md overflow-auto pl-2', clazz)}
>
	{#if !editor}
		<FakeMonacoPlaceHolder autoheight showNumbers={false} {code} {fontSize} />
	{/if}
	<div
		bind:this={divEl}
		style="height: 18px;"
		class="template nonmain-editor rounded-md overflow-clip {!editor ? 'hidden' : ''}"
		bind:clientWidth={width}
	></div>
</div>

<style>
	:global(.template .mtk20) {
		color: black !important;
	}
</style>
