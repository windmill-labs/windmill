<script lang="ts">
	import { getAstNode } from 'svelte-exmarkdown'
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
	import { aiChatManager, AIMode } from '../AIChatManager.svelte'
	import { Check, Play } from 'lucide-svelte'

	const astNode = getAstNode()

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

	// Check if the apply button should be shown
	let showApplyButton = $derived.by(() => {
		if (
			aiChatManager.mode !== AIMode.SCRIPT ||
			!aiChatManager.scriptEditorApplyCode ||
			code === aiChatManager.scriptEditorOptions?.code
		) {
			return false
		}
		return true
	})

	function handleApplyCode() {
		if (code && aiChatManager.scriptEditorApplyCode) {
			aiChatManager.scriptEditorApplyCode(code, { mode: 'apply' })
		}
	}
</script>

<div class="flex flex-col gap-0.5 rounded-lg relative not-prose">
	<div
		class="relative w-full border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden"
	>
		<HighlightCode
			className="p-1"
			code={code ?? ''}
			highlightLanguage={SMART_LANG_TO_HIGHLIGHT_LANG[getSmartLang(language as string)]}
			language={undefined}
			onApplyCode={handleApplyCode}
			{showApplyButton}
			applyButtonIcon={aiChatManager.pendingNewCode ? Check : Play}
		/>
	</div>
</div>
