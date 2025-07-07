<script lang="ts">
	import AIChatDisplay from './AIChatDisplay.svelte'
	import { onDestroy, untrack } from 'svelte'
	import { type ScriptLang } from '$lib/gen'
	import {
		copilotInfo,
		copilotSessionModel,
		dbSchemas,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import { base } from '$lib/base'
	import HideButton from '$lib/components/apps/editor/settingsPanel/HideButton.svelte'
	import { SUPPORTED_CHAT_SCRIPT_LANGUAGES } from './script/core'

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const hasCopilot = $derived($copilotInfo.enabled)
	const disabled = $derived(
		!hasCopilot ||
			(aiChatManager.mode === AIMode.SCRIPT &&
				aiChatManager.scriptEditorOptions?.lang &&
				!SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(aiChatManager.scriptEditorOptions.lang))
	)
	const disabledMessage = $derived(
		!hasCopilot
			? isAdmin
				? `Enable Windmill AI in your [workspace settings](${base}/workspace_settings?tab=ai) to use this chat`
				: 'Ask an admin to enable Windmill AI in this workspace to use this chat'
			: aiChatManager.mode === AIMode.SCRIPT &&
				  aiChatManager.scriptEditorOptions?.lang &&
				  !SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(aiChatManager.scriptEditorOptions.lang)
				? `Windmill AI does not support the ${aiChatManager.scriptEditorOptions.lang} language yet.`
				: ''
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
			mode?: AIMode
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
		} = {}
	) {
		aiChatManager.sendRequest(options)
	}

	function cancel() {
		aiChatManager.cancel()
	}

	const historyManager = aiChatManager.historyManager
	historyManager.init()

	onDestroy(() => {
		cancel()
		historyManager.close()
	})

	let aiChatDisplay: AIChatDisplay | undefined = $state(undefined)

	$effect(() => {
		aiChatManager.listenForDbSchemasChanges($dbSchemas)
	})

	$effect(() => {
		aiChatManager.listenForScriptEditorContextChange(
			$dbSchemas,
			$workspaceStore,
			$copilotSessionModel
		)
	})

	$effect(() => {
		aiChatManager.updateMode(untrack(() => aiChatManager.mode))
	})

	$effect(() => {
		aiChatManager.loadApiTools()
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
			e.preventDefault()
			aiChatManager.toggleOpen()
			aiChatDisplay?.focusInput()
		}
	}}
/>

{#snippet headerLeft()}
	<HideButton
		hidden={false}
		direction="right"
		panelName="AI"
		shortcut="L"
		size="md"
		on:click={() => aiChatManager.toggleOpen()}
	/>
{/snippet}

<AIChatDisplay
	bind:this={aiChatDisplay}
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
	deletePastChat={(id) => {
		historyManager.deletePastChat(id)
	}}
	loadPastChat={(id) => {
		aiChatManager.loadPastChat(id)
	}}
	{cancel}
	askAi={aiChatManager.askAi}
	{headerLeft}
	hasDiff={aiChatManager.scriptEditorOptions &&
		!!aiChatManager.scriptEditorOptions.lastDeployedCode &&
		aiChatManager.scriptEditorOptions.lastDeployedCode !== aiChatManager.scriptEditorOptions.code}
	diffMode={aiChatManager.scriptEditorOptions?.diffMode ?? false}
	{disabled}
	{disabledMessage}
	{suggestions}
></AIChatDisplay>
