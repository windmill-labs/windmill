<script lang="ts">
	import AIChatDisplay from './AIChatDisplay.svelte'
	import { onDestroy, type Snippet } from 'svelte'
	import { type ScriptLang } from '$lib/gen'
	import HistoryManager from './HistoryManager.svelte'
	import {
		copilotInfo,
		copilotSessionModel,
		dbSchemas,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import { aiChatManager } from './AIChatManager.svelte'
	import { base } from '$lib/base'

	interface Props {
		headerLeft?: Snippet
		headerRight?: Snippet
	}

	let { headerLeft, headerRight }: Props = $props()

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const hasCopilot = $derived($copilotInfo.enabled)
	const disabledMessage = $derived(
		hasCopilot
			? ''
			: isAdmin
				? `Enable Windmill AI in your [workspace settings](${base}/workspace_settings?tab=ai) to use this chat`
				: 'Ask an admin to enable Windmill AI in this workspace to use this chat'
	)

	const suggestions = [
		'Where can I see my latest runs?',
		'How do I trigger a script with a webhook endpoint?',
		'How can I connect to a database?',
		'How do I schedule a recurring job?'
	]

	export async function generateStep(moduleId: string, lang: ScriptLang, instructions: string) {
		aiChatManager.generateStep(moduleId, lang, instructions)
	}

	export async function sendRequest(
		options: {
			removeDiff?: boolean
			addBackCode?: boolean
			instructions?: string
			mode?: 'script' | 'flow' | 'navigator'
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
		} = {}
	) {
		aiChatManager.sendRequest(options)
	}

	function cancel() {
		aiChatManager.cancel()
	}

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		aiChatManager.contextManager.addSelectedLinesToContext(lines, startLine, endLine)
	}

	export function focusTextArea() {
		aiChatDisplay?.focusInput()
	}

	const historyManager = new HistoryManager()
	historyManager.init()

	onDestroy(() => {
		cancel()
		historyManager.close()
	})

	let aiChatDisplay: AIChatDisplay | undefined = $state(undefined)

	$effect(() => {
		aiChatManager.initChatEffects($dbSchemas, $workspaceStore, $copilotSessionModel)
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
			e.preventDefault()
			aiChatManager.toggleOpen()
		}
	}}
/>

<AIChatDisplay
	bind:this={aiChatDisplay}
	allowedModes={aiChatManager.allowedModes}
	pastChats={historyManager.getPastChats()}
	bind:selectedContext={
		() => aiChatManager.contextManager.getSelectedContext(),
		(sc) => {
			aiChatManager.scriptEditorOptions && aiChatManager.contextManager.setSelectedContext(sc)
		}
	}
	availableContext={aiChatManager.contextManager.getAvailableContext()}
	messages={aiChatManager.currentReply
		? [
				...aiChatManager.displayMessages,
				{
					role: 'assistant',
					content: aiChatManager.currentReply,
					contextElements: aiChatManager.contextManager
						.getSelectedContext()
						.filter((c) => c.type === 'code')
				}
			]
		: aiChatManager.displayMessages}
	saveAndClear={aiChatManager.saveAndClear}
	deletePastChat={historyManager.deletePastChat}
	loadPastChat={(id) => {
		aiChatManager.loadPastChat(id)
	}}
	{cancel}
	askAi={aiChatManager.askAi}
	{headerLeft}
	{headerRight}
	hasDiff={aiChatManager.scriptEditorOptions &&
		!!aiChatManager.scriptEditorOptions.lastDeployedCode &&
		aiChatManager.scriptEditorOptions.lastDeployedCode !== aiChatManager.scriptEditorOptions.code}
	diffMode={aiChatManager.scriptEditorOptions?.diffMode ?? false}
	disabled={!hasCopilot}
	{disabledMessage}
	{suggestions}
></AIChatDisplay>
