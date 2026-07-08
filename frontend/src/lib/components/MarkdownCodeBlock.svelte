<script lang="ts">
	import { getAstNode } from 'svelte-exmarkdown'
	import HighlightCode from './HighlightCode.svelte'
	import {
		csharp,
		go,
		graphql,
		javascript,
		php,
		plaintext,
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

	// Map a fence language to a known highlighter key, or undefined when the fence
	// is unlabeled/unrecognized — those render as plaintext rather than being
	// mis-colored as TypeScript (this renderer runs on all user markdown).
	function getSmartLang(lang: string | undefined) {
		switch (lang) {
			case 'python':
			case 'python3':
			case 'py':
				return 'python'
			case 'deno':
			case 'nativets':
			case 'bun':
			case 'bunnative':
			case 'typescript':
			case 'ts':
			case 'tsx':
				return 'typescript'
			case 'go':
			case 'golang':
				return 'go'
			case 'shell':
			case 'bash':
			case 'sh':
			case 'zsh':
			case 'console':
				return 'shell'
			case 'frontend':
			case 'javascript':
			case 'js':
			case 'jsx':
			case 'mjs':
			case 'cjs':
				return 'javascript'
			case 'graphql':
				return 'graphql'
			case 'mysql':
			case 'snowflake':
			case 'bigquery':
			case 'oracledb':
			case 'postgresql':
			case 'postgres':
			case 'sql':
				return 'sql'
			case 'php':
				return 'php'
			case 'rust':
			case 'rs':
				return 'rust'
			case 'csharp':
			case 'cs':
				return 'csharp'
			case 'ansible':
			case 'yaml':
			case 'yml':
				return 'yaml'
			default:
				return undefined
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

	let smartLang = $derived(getSmartLang(language))
	let highlightLanguage = $derived(smartLang ? SMART_LANG_TO_HIGHLIGHT_LANG[smartLang] : plaintext)
</script>

<div class="flex flex-col gap-0.5 rounded-md relative not-prose !text-xs">
	<div class="relative w-full bg-surface-secondary rounded-md overflow-hidden">
		{#if renderMermaid && language === 'mermaid'}
			<MermaidDisplay code={code ?? ''} />
		{:else}
			<HighlightCode
				className="px-3 py-2.5 !text-xs"
				code={code ?? ''}
				{highlightLanguage}
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
