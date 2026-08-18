<script lang="ts">
	import { getAstNode } from 'svelte-exmarkdown'
	import MarkdownCodeBlock from '$lib/components/MarkdownCodeBlock.svelte'
	import { AIMode } from '../AIChatManager.svelte'
	import { getAiChatManager } from '../aiChatManagerContext'
	import { Check, Play } from 'lucide-svelte'

	// Chat `pre` renderer: the shared MarkdownCodeBlock plus the chat-only Apply
	// button (wired to the script editor) and mermaid diagrams.
	const aiChatManager = getAiChatManager()

	const astNode = getAstNode()
	let code = $derived(astNode.current.children?.[0]?.children?.[0]?.value)

	let showApplyButton = $derived.by(() => {
		if (
			aiChatManager.mode !== AIMode.SCRIPT ||
			!aiChatManager.scriptEditorApplyCode ||
			code === aiChatManager.scriptEditorOptions?.getCode()
		) {
			return false
		}
		return true
	})

	function handleApplyCode() {
		if (code && aiChatManager.scriptEditorApplyCode) {
			void aiChatManager.applyScriptEditorCode(code, { mode: 'apply' })
		}
	}
</script>

<MarkdownCodeBlock
	{showApplyButton}
	onApplyCode={handleApplyCode}
	applyButtonIcon={aiChatManager.pendingNewCode ? Check : Play}
	renderMermaid
/>
