<script lang="ts">
	import AIChatDisplay from './AIChatDisplay.svelte'
	import { onDestroy, untrack, type Snippet } from 'svelte'
	import { type ScriptLang } from '$lib/gen'
	import HistoryManager from './HistoryManager.svelte'
	import {
		copilotInfo,
		copilotSessionModel,
		dbSchemas,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import { AIChatService } from './AIChatManager.svelte'
	import { base } from '$lib/base'
	import ContextManager from './ContextManager.svelte'

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
		'Where can i see my latest runs?',
		'How do i trigger a script with a webhook endpoint?',
		'How can I connect to a database?',
		'How do I schedule a recurring job?'
	]

	export async function generateStep(moduleId: string, lang: ScriptLang, instructions: string) {
		AIChatService.generateStep(moduleId, lang, instructions)
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
		AIChatService.sendRequest(options)
	}

	function cancel() {
		AIChatService.cancel()
	}

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		AIChatService.contextManager.addSelectedLinesToContext(lines, startLine, endLine)
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
		if (AIChatService.scriptEditorOptions) {
			AIChatService.contextManager.updateAvailableContext(
				AIChatService.scriptEditorOptions,
				$dbSchemas,
				$workspaceStore ?? '',
				!$copilotSessionModel?.model.endsWith('/thinking'),
				untrack(() => AIChatService.contextManager.getSelectedContext())
			)
		}
	})

	$effect(() => {
		AIChatService.displayMessages = ContextManager.updateDisplayMessages(
			untrack(() => AIChatService.displayMessages),
			$dbSchemas
		)
	})

	$effect(() => {
		AIChatService.updateMode(untrack(() => AIChatService.mode))
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
			e.preventDefault()
			AIChatService.toggleOpen()
		}
	}}
/>

<AIChatDisplay
	bind:this={aiChatDisplay}
	allowedModes={AIChatService.allowedModes}
	pastChats={historyManager.getPastChats()}
	bind:selectedContext={
		() => AIChatService.contextManager.getSelectedContext(),
		(sc) => {
			AIChatService.scriptEditorOptions && AIChatService.contextManager.setSelectedContext(sc)
		}
	}
	availableContext={AIChatService.contextManager.getAvailableContext()}
	messages={AIChatService.currentReply
		? [
				...AIChatService.displayMessages,
				{
					role: 'assistant',
					content: AIChatService.currentReply,
					contextElements: AIChatService.contextManager
						.getSelectedContext()
						.filter((c) => c.type === 'code')
				}
			]
		: AIChatService.displayMessages}
	saveAndClear={AIChatService.saveAndClear}
	deletePastChat={historyManager.deletePastChat}
	loadPastChat={(id) => {
		AIChatService.loadPastChat(id)
	}}
	{cancel}
	askAi={AIChatService.askAi}
	{headerLeft}
	{headerRight}
	hasDiff={AIChatService.scriptEditorOptions &&
		!!AIChatService.scriptEditorOptions.lastDeployedCode &&
		AIChatService.scriptEditorOptions.lastDeployedCode !== AIChatService.scriptEditorOptions.code}
	diffMode={AIChatService.scriptEditorOptions?.diffMode ?? false}
	disabled={!hasCopilot}
	{disabledMessage}
	{suggestions}
></AIChatDisplay>
