<script lang="ts">
	import Highlight, { LineNumbers } from 'svelte-highlight'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'
	import go from 'svelte-highlight/languages/go'
	import shell from 'svelte-highlight/languages/shell'
	import graphql from 'svelte-highlight/languages/graphql'
	import javascript from 'svelte-highlight/languages/javascript'
	import sql from 'svelte-highlight/languages/sql'
	import powershell from 'svelte-highlight/languages/powershell'
	import php from 'svelte-highlight/languages/php'
	import rust from 'svelte-highlight/languages/rust'
	import csharp from 'svelte-highlight/languages/csharp'
	import yaml from 'svelte-highlight/languages/yaml'
	import java from 'svelte-highlight/languages/java'
	import ruby from 'svelte-highlight/languages/ruby'
	import r from 'svelte-highlight/languages/r'
	import type { Script } from '$lib/gen'
	import { Button } from './common'
	import CopyButton from './common/button/CopyButton.svelte'
	import ScrollableX from './common/ScrollableX.svelte'
	import { copyToClipboard } from '$lib/utils'
	import { ClipboardCopy } from 'lucide-svelte'
	import HighlightTheme from './HighlightTheme.svelte'
	import { json, type LanguageType } from 'svelte-highlight/languages'

	interface Props {
		code?: string
		language: Script['language'] | 'bunnative' | 'frontend' | 'json' | undefined
		highlightLanguage?: LanguageType<string> | undefined
		lines?: boolean
		className?: string
		onApplyCode?: () => void
		showApplyButton?: boolean
		applyButtonIcon?: typeof ClipboardCopy
		/** Keep the copy/apply buttons hidden until the block is hovered, and render them subtly. */
		buttonsOnHover?: boolean
		/** Wrap the code in ScrollableX (native scroll, subtle hover-revealed scrollbar) for horizontal overflow. */
		customScrollbarX?: boolean
	}

	let {
		code = '',
		language,
		highlightLanguage = undefined,
		lines = false,
		className = '',
		onApplyCode = undefined,
		showApplyButton = false,
		applyButtonIcon = undefined,
		buttonsOnHover = false,
		customScrollbarX = false
	}: Props = $props()

	const hoverButtonClasses = buttonsOnHover
		? 'opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity duration-150'
		: ''

	function getLang(lang: Script['language'] | 'bunnative' | 'frontend' | 'json' | undefined) {
		switch (lang) {
			case 'python3':
				return python
			case 'deno':
				return typescript
			case 'nativets':
				return typescript
			case 'bun':
				return typescript
			case 'bunnative':
				return typescript
			case 'go':
				return go
			case 'bash':
				return shell
			case 'frontend':
				return javascript
			case 'graphql':
				return graphql
			case 'mysql':
				return sql
			case 'postgresql':
				return sql
			case 'snowflake':
				return sql
			case 'bigquery':
				return sql
			case 'oracledb':
				return sql
			case 'duckdb':
				return sql
			case 'powershell':
				return powershell
			case 'php':
				return php
			case 'rust':
				return rust
			case 'csharp':
				return csharp
			case 'nu':
				return python
			case 'ansible':
				return yaml
			case 'java':
				return java
			case 'ruby':
				return ruby
			case 'rlang':
				return r
			case 'json':
				return json
			// for related places search: ADD_NEW_LANG
			default:
				return typescript
		}
	}

	const lang = $derived(highlightLanguage ?? getLang(language))
</script>

<HighlightTheme />

<div class="relative group">
	{#if buttonsOnHover}
		<!-- Chat/markdown code blocks: subtle CopyButton on a surface chip. -->
		<CopyButton
			value={code}
			class="absolute top-2 right-2 z-20 bg-surface rounded-md {hoverButtonClasses}"
		/>
	{:else}
		<Button
			wrapperClasses="absolute top-2 right-2 z-20"
			onclick={() => copyToClipboard(code)}
			color="light"
			size="xs2"
			startIcon={{ icon: ClipboardCopy }}
			iconOnly
			title="Copy to clipboard"
		/>
	{/if}
	{#if showApplyButton}
		<Button
			wrapperClasses="absolute top-2 right-10 z-20 {hoverButtonClasses}"
			onclick={onApplyCode}
			color="light"
			size="xs2"
			startIcon={{
				icon: applyButtonIcon
			}}
			iconOnly
			title="Apply code"
		/>
	{/if}
	{#if customScrollbarX}
		<ScrollableX>
			{@render codeBody()}
		</ScrollableX>
	{:else}
		<div class="overflow-x-auto">
			{@render codeBody()}
		</div>
	{/if}
</div>

{#snippet codeBody()}
	{#if code?.length < 10000}
		{#if !lines}
			<Highlight class="nowrap {className}" language={lang} {code} />
		{:else}
			<Highlight class="nowrap {className}" language={lang} {code} let:highlighted>
				<LineNumbers {highlighted} />
			</Highlight>
		{/if}
	{:else}
		<pre class="overflow-auto max-h-screen text-xs {className}"
			><code class="language-{language}">{code}</code></pre
		>
	{/if}
{/snippet}
