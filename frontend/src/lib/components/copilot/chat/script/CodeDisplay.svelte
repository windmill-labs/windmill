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
	import { getCurrentModel } from '$lib/aiStore'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { AIProvider } from '$lib/gen/types.gen'

	const astNode = getAstNode()

	// Providers that support diff-based editing
	const DIFF_BASED_EDIT_PROVIDERS: AIProvider[] = ['openai', 'anthropic', 'googleai', 'azure_openai']

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
		// Only show in script mode
		if (aiChatManager.mode !== AIMode.SCRIPT) {
			return false
		}

		// Only show if we have an applyCode function available
		if (!aiChatManager.scriptEditorApplyCode) {
			return false
		}

		// Only show for non-diff-based providers
		try {
			const currentModel = getCurrentModel()
			return !DIFF_BASED_EDIT_PROVIDERS.includes(currentModel.provider)
		} catch (e) {
			return false
		}
	})

	function handleApplyCode() {
		if (code && aiChatManager.scriptEditorApplyCode) {
			aiChatManager.scriptEditorApplyCode(code, { applyAll: true, mode: 'apply' })
		}
	}
</script>

<div
	class="flex flex-col not-prose relative w-full border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden"
>
	{#if showApplyButton}
		<div class="flex items-center justify-between p-2 bg-gray-50 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-600">
			<span class="text-xs text-gray-600 dark:text-gray-400">Code</span>
			<Button
				size="xs"
				color="green"
				onclick={handleApplyCode}
			>
				Apply code
			</Button>
		</div>
	{/if}
	<HighlightCode
		class="p-1"
		code={code ?? ''}
		highlightLanguage={SMART_LANG_TO_HIGHLIGHT_LANG[getSmartLang(language as string)]}
		language={undefined}
	/>
</div>
