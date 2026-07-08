<script lang="ts">
	import { getAstNode } from 'svelte-exmarkdown'
	import HighlightCode from './HighlightCode.svelte'
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
	import type { ClipboardCopy } from 'lucide-svelte'
	import MermaidDisplay from './copilot/chat/script/MermaidDisplay.svelte'

	// Shared `pre` renderer for fenced code blocks in svelte-exmarkdown markdown.
	// Chat-agnostic: gives every markdown surface the highlighted panel + subtle
	// hover copy button + custom horizontal scrollbar. The AI chat wraps this to
	// add its Apply button (via the apply props) and mermaid diagrams.
	let {
		showApplyButton = false,
		onApplyCode = undefined,
		applyButtonIcon = undefined,
		// Render ```mermaid blocks as diagrams. Off by default so untrusted
		// markdown surfaces don't execute mermaid — the chat opts in.
		renderMermaid = false
	}: {
		showApplyButton?: boolean
		onApplyCode?: () => void
		applyButtonIcon?: typeof ClipboardCopy
		renderMermaid?: boolean
	} = $props()

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
</script>

<div class="flex flex-col gap-0.5 rounded-md relative not-prose !text-xs">
	<div class="relative w-full bg-surface-secondary rounded-md overflow-hidden">
		{#if renderMermaid && language === 'mermaid'}
			<MermaidDisplay code={code ?? ''} />
		{:else}
			<HighlightCode
				className="px-3 py-2.5 !text-xs"
				code={code ?? ''}
				highlightLanguage={SMART_LANG_TO_HIGHLIGHT_LANG[getSmartLang(language as string)]}
				language={undefined}
				{onApplyCode}
				{showApplyButton}
				{applyButtonIcon}
				buttonsOnHover
				customScrollbarX
			/>
		{/if}
	</div>
</div>
