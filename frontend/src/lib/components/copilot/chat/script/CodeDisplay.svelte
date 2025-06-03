<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getAstNode } from 'svelte-exmarkdown'
	import { editor as meditor } from 'monaco-editor'
	import { getContext, untrack } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { initializeVscode } from '$lib/components/vscode'
	import type { DisplayMessage } from '../shared'
	import type { ContextElement } from '../context'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		csharp,
		go,
		graphql,
		javascript,
		php,
		python,
		rust,
		shell,
		sql,
		typescript,
		yaml
	} from 'svelte-highlight/languages'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { AIChatService } from '../AIChatManager.svelte'

	const astNode = getAstNode()

	const { message } = getContext<{ message: DisplayMessage }>('AssistantMessageContext')

	let codeContext = $derived(
		message.role === 'assistant' &&
			(message.contextElements?.find((e) => e.type === 'code') as
				| Extract<ContextElement, { type: 'code' }>
				| undefined)
	)

	function getSmartLang(lang: string) {
		switch (lang) {
			case 'python':
			case 'python3':
				return 'python'
			case 'deno':
			case 'nativets':
			case 'bun':
			case 'bunnative':
			case 'typescript':
				return 'typescript'
			case 'go':
				return 'go'
			case 'shell':
			case 'bash':
				return 'shell'
			case 'frontend':
			case 'javascript':
				return 'javascript'
			case 'graphql':
				return 'graphql'
			case 'mysql':
			case 'snowflake':
			case 'bigquery':
			case 'oracledb':
			case 'powershell':
			case 'postgresql':
			case 'sql':
				return 'sql'
			case 'php':
				return 'php'
			case 'rust':
				return 'rust'
			case 'csharp':
				return 'csharp'
			case 'ansible':
			case 'yaml':
				return 'yaml'
			default:
				return 'typescript'
		}
	}

	const SMART_LANG_TO_HIGHLIGHT_LANG = {
		python: python,
		typescript: typescript,
		go: go,
		shell: shell,
		javascript: javascript,
		graphql: graphql,
		sql: sql,
		php: php,
		rust: rust,
		csharp: csharp,
		yaml: yaml
	}

	let code = $derived(astNode.current.children?.[0]?.children?.[0]?.value)

	let language = $derived(
		(astNode.current.children?.[0]?.properties?.class as string | undefined)?.split('-')[1]
	)

	let loading = $state(true)
	$effect(() => {
		// we only want to trigger when astNode offset is updated not currentReply, otherwise as there is some delay on the offset update, loading would be set to false too early
		const completeReply = untrack(() => AIChatService.currentReply)
		if (!loading || completeReply.length > (astNode.current.position?.end.offset ?? 0)) {
			loading = false
		}
	})

	let diffEl: HTMLDivElement | undefined = $state()
	let diffEditor: meditor.IStandaloneDiffEditor | undefined = $state()
	async function setDiffEditor(diffEl: HTMLDivElement) {
		if (!codeContext) {
			return
		}
		await initializeVscode()

		diffEditor = meditor.createDiffEditor(diffEl, {
			automaticLayout: true,
			renderSideBySide: false,
			hideUnchangedRegions: {
				enabled: true
			},
			originalEditable: false,
			readOnly: true,
			renderGutterMenu: false,
			renderOverviewRuler: false,
			scrollBeyondLastLine: false,
			overviewRulerLanes: 0,
			lineNumbersMinChars: 0,
			lightbulb: {
				enabled: meditor.ShowLightbulbIconMode.Off
			},
			scrollbar: {
				alwaysConsumeMouseWheel: false
			}
		})

		diffEditor.setModel({
			original: meditor.createModel(codeContext.content, scriptLangToEditorLang(codeContext.lang)),
			modified: meditor.createModel(code ?? '', language ? getSmartLang(language) : undefined)
		})

		const originalEditor = diffEditor.getOriginalEditor()
		const modifiedEditor = diffEditor.getModifiedEditor()

		originalEditor.onDidContentSizeChange((e) => {
			diffEl.style.height = `${e.contentHeight}px`
		})

		modifiedEditor.onDidContentSizeChange((e) => {
			diffEl.style.height = `${e.contentHeight}px`
		})

		updateModifiedModel(code ?? '')
	}

	function updateModifiedModel(code: string) {
		const modified = diffEditor?.getModifiedEditor()

		if (!modified) return

		const modifiedModel = modified.getModel()
		if (modifiedModel) {
			modifiedModel.setValue(code ?? '')
		}
	}
	$effect(() => updateModifiedModel(code ?? ''))

	$effect(() => {
		diffEl &&
			language &&
			codeContext &&
			getSmartLang(codeContext.lang) === getSmartLang(language) &&
			setDiffEditor(diffEl)
	})
</script>

<div class="flex flex-col gap-0.5 rounded-lg relative not-prose">
	{#if AIChatService.canApplyCode}
		<div class="flex justify-end items-end">
			<Button
				color="dark"
				size="xs2"
				on:click={() => {
					AIChatService.scriptEditorApplyCode?.(code ?? '')
				}}
			>
				Apply
			</Button>
		</div>
	{/if}

	<div
		class="relative w-full border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden"
	>
		{#if (loading && !code) || !language}
			<div class="flex flex-row gap-1 p-2 items-center justify-center">
				<Loader2 class="w-4 h-4 animate-spin" /> Generating code...
			</div>
		{:else if !loading && codeContext && getSmartLang(codeContext.lang) === getSmartLang(language)}
			<div bind:this={diffEl} class="w-full h-full"></div>
		{:else}
			<HighlightCode
				class="p-1"
				code={code ?? ''}
				highlightLanguage={SMART_LANG_TO_HIGHLIGHT_LANG[getSmartLang(language)]}
				language={undefined}
			/>
		{/if}
	</div>
</div>
