<script module>
	let cssClassesLoaded = $state(false)
	let tailwindClassesLoaded = $state(false)

	import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-standalone-css-language-features'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import '@codingame/monaco-vscode-standalone-html-language-features'
	import {
		editor as meditor,
		KeyCode,
		KeyMod,
		Uri as mUri,
		languages,
		type IRange,
		type IDisposable
	} from 'monaco-editor'

	languages.typescript.javascriptDefaults.setCompilerOptions({
		target: languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noSemanticValidation: false,
		noLib: true,
		moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs
	})
	function setDiagnosticsOptions() {
		languages.typescript.javascriptDefaults.setDiagnosticsOptions({
			noSemanticValidation: false,
			noSyntaxValidation: false,
			noSuggestionDiagnostics: false,
			diagnosticCodesToIgnore: [1108]
		})
	}
	setDiagnosticsOptions()
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
</script>

<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'
	// import {
	// 	editor as meditor,
	// 	KeyCode,
	// 	KeyMod,
	// 	Uri as mUri,
	// 	languages,
	// 	type IRange,
	// 	type IDisposable
	// } from 'monaco-editor'

	import { allClasses } from './apps/editor/componentsPanel/cssUtils'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import libStdContent from '$lib/es6.d.ts.txt?raw'
	import domContent from '$lib/dom.d.ts.txt?raw'
	import { initializeVscode, keepModelAroundToAvoidDisposalOfWorkers } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import { vimMode } from '$lib/stores'
	import { initVim } from './monaco_keybindings'
	import FakeMonacoPlaceHolder from './FakeMonacoPlaceHolder.svelte'
	// import { createConfiguredEditor } from 'vscode/monaco'
	// import type { IStandaloneCodeEditor } from 'vscode/vscode/vs/editor/standalone/browser/standaloneCodeEditor'

	let divEl: HTMLDivElement | null = null
	let editor = $state<meditor.IStandaloneCodeEditor | null>(null)
	let model: meditor.ITextModel

	let statusDiv = $state<Element | null>(null)
	let width = $state(0)
	let initialized = $state(false)
	let suggestion = $state('')
	let placeholderVisible = $state(false)
	let mounted = $state(false)

	let valueAfterDispose: string | undefined = undefined
	let {
		lang,
		code = $bindable(),
		hash = createHash(),
		cmdEnterAction,
		formatAction,
		automaticLayout = true,
		extraLib = '',
		placeholder = '',
		disableSuggestions = false,
		disableLinting = false,
		hideLineNumbers = false,
		shouldBindKey = true,
		autoHeight = false,
		fixedOverflowWidgets = true,
		small = false,
		domLib = false,
		autofocus = false,
		allowVim = false,
		tailwindClasses = [],
		class: className = '',
		loadAsync = false
	} = $props<{
		lang: string
		code?: string
		hash?: string
		cmdEnterAction?: () => void
		formatAction?: () => void
		automaticLayout?: boolean
		extraLib?: string
		placeholder?: string
		disableSuggestions?: boolean
		disableLinting?: boolean
		hideLineNumbers?: boolean
		shouldBindKey?: boolean
		autoHeight?: boolean
		fixedOverflowWidgets?: boolean
		small?: boolean
		domLib?: boolean
		autofocus?: boolean
		allowVim?: boolean
		tailwindClasses?: string[]
		class?: string
		loadAsync?: boolean
	}>()

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

	export function getCode(): string {
		if (valueAfterDispose != undefined) {
			return valueAfterDispose
		}
		return editor?.getValue() ?? ''
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
		}
	}

	export function setCode(ncode: string): void {
		if (ncode != code) {
			code = ncode
		}
		editor?.setValue(ncode)
	}

	function updateCode(): boolean {
		const ncode = getCode()
		if (code == ncode) {
			return false
		}
		code = ncode
		return true
	}

	function updatePlaceholderVisibility(value: string) {
		if (!value) {
			placeholderVisible = true
			return
		}
		placeholderVisible = value.trim() === ''
	}

	export function format() {
		if (editor) {
			updateCode()
			editor.getAction('editor.action.formatDocument')?.run()
			if (formatAction) {
				formatAction()
				updateCode()
			}
		}
	}

	export function focus() {
		editor?.focus()
	}

	export function getSelectedLines(): string | undefined {
		if (editor) {
			const selection = editor.getSelection()
			if (selection) {
				const range: IRange = {
					startLineNumber: selection.startLineNumber,
					startColumn: 1,
					endLineNumber: selection.endLineNumber + 1,
					endColumn: 1
				}
				return editor.getModel()?.getValueInRange(range)
			}
		}
	}

	export function onDidChangeCursorSelection(f: (e: meditor.ICursorSelectionChangedEvent) => void) {
		if (editor) {
			return editor.onDidChangeCursorSelection(f)
		}
	}

	export function show(): void {
		divEl?.classList.remove('hidden')
	}

	export function hide(): void {
		divEl?.classList.add('hidden')
	}

	export function setSuggestion(value: string): void {
		suggestion = value
	}

	let disableTabCond: meditor.IContextKey<boolean> | undefined

	$effect(() => {
		disableTabCond?.set(!code && !!suggestion)
	})

	let vimDisposable: IDisposable | undefined = undefined

	$effect(() => {
		if (allowVim && editor !== null && $vimMode && statusDiv) {
			onVimMode()
		}
	})

	$effect(() => {
		if (!$vimMode && vimDisposable) {
			onVimDisable()
		}
	})

	function onVimDisable() {
		vimDisposable?.dispose()
	}

	function onVimMode() {
		if (editor && statusDiv) {
			vimDisposable = initVim(editor, statusDiv, () => {
				console.log('vim save not possible for simple editor')
			})
		}
	}

	function updateModelAndOptions() {
		const model = editor?.getModel()
		if (model) {
			// Switch language if it changed
			if (model.getLanguageId() !== lang) {
				const currentCode = model.getValue()
				const uri = `file:///${hash}.${langToExt(lang)}`
				const oldModel = model
				const newModel = meditor.createModel(currentCode, lang, mUri.parse(uri))
				editor?.setModel(newModel)
				oldModel.dispose()
			}

			// Update editor options for suggestions, validation decorations, and line numbers
			editor?.updateOptions({
				quickSuggestions: disableSuggestions
					? { other: false, comments: false, strings: false }
					: { other: true, comments: true, strings: true },
				suggestOnTriggerCharacters: !disableSuggestions,
				wordBasedSuggestions: disableSuggestions ? 'off' : 'matchingDocuments',
				parameterHints: { enabled: !disableSuggestions },
				suggest: {
					showIcons: !disableSuggestions,
					showSnippets: !disableSuggestions,
					showKeywords: !disableSuggestions,
					showWords: !disableSuggestions,
					snippetsPreventQuickSuggestions: disableSuggestions
				},
				lineNumbers: hideLineNumbers ? 'off' : 'on',
				lineDecorationsWidth: hideLineNumbers ? 0 : 6,
				lineNumbersMinChars: hideLineNumbers ? 0 : 2,
				// Hide validation squiggles and decorations
				renderValidationDecorations: disableLinting ? 'off' : 'on',
				// Hide the validation margin indicators
				hideCursorInOverviewRuler: disableLinting,
				overviewRulerBorder: !disableLinting,
				overviewRulerLanes: disableLinting ? 0 : 3
			})
		}
	}

	$effect(() => {
		if (editor !== null && (lang || disableLinting || disableSuggestions || hideLineNumbers)) {
			updateModelAndOptions()
		}
	})

	let fontSize = $derived(small ? 12 : 14)

	async function loadMonaco() {
		await initializeVscode()
		initialized = true

		// if (lang === 'javascript') {
		// 	languages.typescript.javascriptDefaults.setCompilerOptions({
		// 		target: languages.typescript.ScriptTarget.Latest,
		// 		allowNonTsExtensions: true,
		// 		noSemanticValidation: false,
		// 		noLib: true,
		// 		moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs
		// 	})
		// 	languages.typescript.javascriptDefaults.setDiagnosticsOptions({
		// 		noSemanticValidation: false,
		// 		noSyntaxValidation: false,
		// 		noSuggestionDiagnostics: false,
		// 		diagnosticCodesToIgnore: [1108]
		// 	})
		// } else if (lang === 'json') {
		// 	languages.json.jsonDefaults.setDiagnosticsOptions({
		// 		validate: true,
		// 		allowComments: false,
		// 		schemas: [],
		// 		enableSchemaRequest: true
		// 	})
		// }
		try {
			model = meditor.createModel(code, lang, mUri.parse(uri))
		} catch (err) {
			console.log('model already existed', err)
			const nmodel = meditor.getModel(mUri.parse(uri))
			if (!nmodel) {
				throw err
			}
			model = nmodel
		}
		model.updateOptions(updateOptions)
		// let widgets: HTMLElement | undefined =
		// 	document.getElementById('monaco-widgets-root') ?? undefined

		if (!divEl) {
			return
		}
		try {
			editor = meditor.create(divEl as HTMLDivElement, {
				...editorConfig(code, lang, automaticLayout, fixedOverflowWidgets),
				model,
				lineDecorationsWidth: 6,
				lineNumbersMinChars: 2,
				fontSize: fontSize,
				quickSuggestions: disableSuggestions
					? { other: false, comments: false, strings: false }
					: { other: true, comments: true, strings: true },
				suggestOnTriggerCharacters: !disableSuggestions,
				wordBasedSuggestions: disableSuggestions ? 'off' : 'matchingDocuments',
				parameterHints: { enabled: !disableSuggestions },
				suggest: {
					showIcons: !disableSuggestions,
					showSnippets: !disableSuggestions,
					showKeywords: !disableSuggestions,
					showWords: !disableSuggestions,
					snippetsPreventQuickSuggestions: disableSuggestions
				}
			})
		} catch (e) {
			console.error('Error loading monaco:', e)
			return
		}
		keepModelAroundToAvoidDisposalOfWorkers()

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			suggestion = ''
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				if (updateCode()) {
					dispatch('change', { code })
				}
			}, 200)
		})

		editor.onDidFocusEditorText(() => {
			if (!editor) return
			dispatch('focus')
			loadExtraLib()

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				updateCode()
				shouldBindKey && format && format()
			})

			disableTabCond = editor.createContextKey('disableTabCond', !code)
			editor.addCommand(KeyCode.Tab, function () {}, 'disableTabCond')

			editor.addCommand(KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.Digit7, function () {
				// CMD + slash (toggle comment) on some EU keyboards
				editor?.trigger('keyboard', 'editor.action.commentLine', {})
			})
		})

		if (autoHeight) {
			const updateHeight = () => {
				if (!editor) return
				const contentHeight = Math.min(1000, editor.getContentHeight())
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					editor.layout({ width, height: contentHeight })
				} finally {
				}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			if (!editor) return
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				updateCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.Enter, function () {
				updateCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			dispatch('blur')
			updateCode()
		})

		if (lang === 'css' && !cssClassesLoaded) {
			cssClassesLoaded = true
			addCSSClassCompletions()
		}

		if (lang === 'tailwindcss' && !tailwindClassesLoaded) {
			languages.register({ id: 'tailwindcss' })
			tailwindClassesLoaded = true
			addTailwindClassCompletions()
		}

		if (placeholder) {
			editor.onDidChangeModelContent(() => {
				if (!editor) return
				const value = editor.getValue()
				updatePlaceholderVisibility(value)
			})
		}
	}

	function addCSSClassCompletions() {
		languages.registerCompletionItemProvider('css', {
			provideCompletionItems: function (model, position, context, token) {
				const word = model.getWordUntilPosition(position)
				const range = {
					startLineNumber: position.lineNumber,
					startColumn: word.startColumn,
					endLineNumber: position.lineNumber,
					endColumn: word.endColumn
				}

				if (word && word.word) {
					const currentWord = word.word

					const suggestions = allClasses
						.filter((className) => className.includes(currentWord))
						.map((className) => ({
							label: className,
							kind: languages.CompletionItemKind.Class,
							insertText: className,
							documentation: 'Custom CSS class',
							range: range
						}))

					return { suggestions }
				}

				return { suggestions: [] }
			}
		})
	}

	function addTailwindClassCompletions() {
		// Define a custom word definition for Tailwind classes
		languages.setMonarchTokensProvider('tailwindcss', {
			tokenizer: {
				root: [[/[a-zA-Z0-9-]+/, 'tailwind-class']]
			}
		})

		languages.registerCompletionItemProvider('tailwindcss', {
			triggerCharacters: ['-'],
			provideCompletionItems: function (model, position, context, token) {
				const wordUntilPosition = model.getWordUntilPosition(position)
				const lineContent = model.getLineContent(position.lineNumber)

				// Get the text from the start of the line to the cursor
				const textUntilPosition = lineContent.substring(0, position.column - 1)
				// Find the last space before the cursor
				const lastSpaceIndex = textUntilPosition.lastIndexOf(' ')
				const startColumn = lastSpaceIndex === -1 ? 1 : lastSpaceIndex + 2

				const range = {
					startLineNumber: position.lineNumber,
					startColumn: startColumn,
					endLineNumber: position.lineNumber,
					endColumn: position.column
				}

				const currentWord = wordUntilPosition.word

				const suggestions = tailwindClasses
					.filter((className) => className.includes(currentWord))
					.map((className) => ({
						label: className,
						kind: languages.CompletionItemKind.Class,
						insertText: className,
						documentation: 'Tailwind CSS class',
						range: range,
						preselect: true
					}))

				return { suggestions }
			}
		})
	}

	let previousExtraLib = undefined
	function loadExtraLib() {
		if (lang == 'javascript') {
			const stdLib = { content: libStdContent, filePath: 'es6.d.ts' }
			const libs = [stdLib]
			if (domLib) {
				const domDTS = { content: domContent, filePath: 'dom.d.ts' }
				libs.push(domDTS)
			}
			if (extraLib != '') {
				libs.push({
					content: extraLib,
					filePath: 'windmill.d.ts'
				})
				if (previousExtraLib == extraLib) {
					return
				}
				previousExtraLib = extraLib
			}
			languages.typescript.javascriptDefaults.setExtraLibs(libs)
		}
	}

	onMount(async () => {
		if (BROWSER) {
			if (loadAsync) {
				setTimeout(async () => {
					await loadMonaco()
					mounted = true
					if (autofocus) setTimeout(() => focus(), 0)
				}, 0)
			} else {
				await loadMonaco()
				mounted = true
				if (autofocus) setTimeout(() => focus(), 0)
			}
		}
	})

	$effect(() => {
		if (mounted && extraLib && initialized) {
			loadExtraLib()
		}
	})

	onDestroy(() => {
		try {
			valueAfterDispose = getCode()
			vimDisposable?.dispose()
			model && model.dispose()
			editor && editor.dispose()
		} catch (err) {}
	})

	export function setCursorToEnd(): void {
		if (editor) {
			const lastLine = editor.getModel()?.getLineCount() ?? 1
			const lastColumn = editor.getModel()?.getLineMaxColumn(lastLine) ?? 1
			editor.setPosition({ lineNumber: lastLine, column: lastColumn })
			editor.focus()
		}
	}

	updatePlaceholderVisibility(code)
</script>

<EditorTheme />
{#if editor && suggestion && code.length === 0}
	<div
		class="absolute top-[0.05rem] left-[2.05rem] z-10 text-sm text-[#0007] italic font-mono dark:text-[#ffffff56] text-ellipsis overflow-hidden whitespace-nowrap"
		style={`max-width: calc(${width}px - 2.05rem)`}
	>
		{suggestion}
	</div>
{/if}

{#if !editor}
	<FakeMonacoPlaceHolder
		{code}
		autoheight
		lineNumbersWidth={(23 * fontSize) / 14}
		lineNumbersOffset={fontSize == 14 ? -8 : -11}
		{fontSize}
	/>
{/if}

<div
	bind:this={divEl}
	class="relative {className} {!editor ? 'hidden' : ''} editor simple-editor {!allowVim
		? 'nonmain-editor'
		: ''}"
	bind:clientWidth={width}
>
	{#if placeholder}
		<div
			id="placeholder"
			class="absolute left-[24px] text-gray-500 text-sm pointer-events-none font-mono z-10 {placeholderVisible
				? ''
				: 'hidden'}"
		>
			{@html placeholder}
		</div>
	{/if}
</div>
{#if allowVim && vimMode}
	<div class="fixed bottom-0 z-30" bind:this={statusDiv}></div>
{/if}

<style lang="postcss">
	.editor {
		@apply rounded-lg p-0;
	}

	.small-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 26vh;
	}

	.few-lines-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 100px;
	}
</style>
