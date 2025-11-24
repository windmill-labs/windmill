<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import { Loader2 } from 'lucide-svelte'
	import { initCss } from '../../utils'
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte'
	import ChatInput from '$lib/components/chat/ChatInput.svelte'
	import { parseStreamDeltas } from '$lib/components/chat/utils'
	import { randomUUID } from '$lib/components/flows/conversations/FlowChatManager.svelte'

	interface Message {
		id: string
		role: 'user' | 'assistant'
		content: string
	}

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'chatcomponent'> | undefined
		recomputeIds?: string[] | undefined
		render: boolean
		errorHandledByComponent?: boolean
	}

	let {
		id,
		componentInput,
		configuration,
		customCss = undefined,
		recomputeIds = undefined,
		render,
		errorHandledByComponent = $bindable(false)
	}: Props = $props()

	const { worldStore, app, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	// Initialize outputs
	let outputs = initOutput($worldStore, id, {
		result: undefined as any,
		loading: false,
		jobId: undefined as string | undefined,
		messages: [] as Message[],
		userMessage: '' as string // Output for evalv2 field
	})

	// Resolve configuration
	let resolvedConfig = $state(
		initConfig(components['chatcomponent'].initialData.configuration, configuration)
	)

	// Initialize CSS
	let css = $state(initCss($app.css?.chatcomponent, customCss))

	// State
	let runnableComponent: RunnableComponent | undefined = $state()
	let runnableWrapper: RunnableWrapper | undefined = $state()
	let loading = $state(false)
	let result: any = $state(undefined)
	let messages: Message[] = $state([])
	let inputValue = $state('')
	let messagesContainer: HTMLDivElement | undefined = $state()

	// Streaming state management
	let currentStreamingMessageIndex: number | undefined = $state(undefined)
	let accumulatedContent = $state('')

	// Generate stable memory_id for chat session (for agent memory persistence)
	let chatMemoryId = $state(randomUUID())

	// Register component control for programmatic access
	$componentControl[id] = {
		sendMessage: (message: string) => {
			if (message && !loading) {
				inputValue = message
				handleSend()
			}
		}
	}

	// Auto-scroll to bottom when messages change
	$effect(() => {
		if (messages.length > 0 && messagesContainer) {
			setTimeout(() => {
				messagesContainer?.scrollTo({
					top: messagesContainer.scrollHeight,
					behavior: 'smooth'
				})
			}, 50)
		}
	})

	// Handle streaming updates
	function handleStreamUpdate(e: CustomEvent<{ id: string; result_stream: string }>) {
		const streamContent = e.detail.result_stream

		const parsed = parseStreamDeltas(streamContent)
		if (parsed.content) {
			accumulatedContent = parsed.content
		} else {
			accumulatedContent = streamContent
		}

		// Update or create streaming message
		if (currentStreamingMessageIndex !== undefined) {
			messages = messages.map((msg, idx) =>
				idx === currentStreamingMessageIndex ? { ...msg, content: accumulatedContent } : msg
			)
		} else {
			const assistantMessage: Message = {
				id: randomUUID(),
				role: 'assistant',
				content: accumulatedContent
			}
			messages = [...messages, assistantMessage]
			currentStreamingMessageIndex = messages.length - 1
		}
	}

	// Handle job completion
	function handleJobComplete(e: CustomEvent<{ id: string; result: any }>) {
		const finalResult = e.detail.result

		// Extract final content
		let finalContent = ''
		if (typeof finalResult === 'string') {
			finalContent = finalResult
		} else if (finalResult && typeof finalResult === 'object' && 'output' in finalResult) {
			finalContent =
				typeof finalResult.output === 'string'
					? finalResult.output
					: JSON.stringify(finalResult.output, null, 2)
		} else {
			finalContent = JSON.stringify(finalResult, null, 2)
		}

		// If we were streaming, update the message with final result to ensure completeness
		if (currentStreamingMessageIndex !== undefined && finalContent) {
			messages = messages.map((msg, idx) =>
				idx === currentStreamingMessageIndex ? { ...msg, content: finalContent } : msg
			)
		}
		// If not streaming, create new message
		else if (finalContent.length > 0) {
			messages = [
				...messages,
				{
					id: randomUUID(),
					role: 'assistant',
					content: finalContent
				}
			]
		}

		// Finalize streaming
		currentStreamingMessageIndex = undefined
		accumulatedContent = ''
	}

	// Handle job error
	function handleJobError(e: CustomEvent<{ id: string; error: any }>) {
		const error = e.detail.error

		// Add error message
		messages = [
			...messages,
			{
				id: randomUUID(),
				role: 'assistant',
				content: `Error: ${error.message || JSON.stringify(error)}`
			}
		]

		// Reset streaming state
		currentStreamingMessageIndex = undefined
		accumulatedContent = ''
	}

	// Handle send message
	async function handleSend() {
		if (!inputValue.trim() || loading) return

		const userMessage = inputValue.trim()
		inputValue = ''

		// Add user message to chat
		const newUserMessage: Message = {
			id: randomUUID(),
			role: 'user',
			content: userMessage
		}
		messages = [...messages, newUserMessage]

		// Reset streaming state for new message
		currentStreamingMessageIndex = undefined
		accumulatedContent = ''

		// Update output so evalv2 field can reference it
		outputs.userMessage.set(userMessage)

		// Trigger the runnable
		if (!runnableComponent) {
			runnableWrapper?.handleSideEffect(true)
		} else {
			await runnableComponent?.runComponent()
		}
	}

	// Handle enter key
	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			handleSend()
		}
	}

	$effect(() => {
		errorHandledByComponent = resolvedConfig?.onError?.selected !== 'errorOverlay'
	})
</script>

<InitializeComponent {id} />

{#each Object.keys(components['chatcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.chatcomponent}
	/>
{/each}

<RunnableWrapper
	bind:this={runnableWrapper}
	bind:runnableComponent
	bind:loading
	bind:result
	{componentInput}
	{id}
	{recomputeIds}
	{outputs}
	doOnSuccess={resolvedConfig.onSuccess}
	doOnError={resolvedConfig.onError}
	on:streamupdate={handleStreamUpdate}
	on:done={handleJobComplete}
	on:doneError={handleJobError}
	{errorHandledByComponent}
	autoRefresh={false}
	{render}
	extraQueryParams={{ memory_id: chatMemoryId }}
>
	{#if render}
		<div
			class={twMerge(
				'flex flex-col h-full border rounded-lg bg-surface',
				css?.container?.class,
				'wm-chat-container'
			)}
			style={css?.container?.style}
		>
			<!-- Messages Container -->
			<div
				bind:this={messagesContainer}
				class={twMerge(
					'flex-1 overflow-y-auto p-4 bg-background',
					css?.messagesContainer?.class,
					'wm-chat-messages'
				)}
				style={css?.messagesContainer?.style}
			>
				{#if messages.length === 0}
					<div class="flex items-center justify-center h-full text-tertiary text-sm">
						No messages yet. Start a conversation!
					</div>
				{:else}
					<div class="w-full space-y-4 xl:max-w-7xl mx-auto">
						{#each messages as message (message.id)}
							<ChatMessage
								role={message.role}
								content={message.content}
								enableMarkdown={true}
								enableS3Display={true}
								customCss={{
									userMessage: css?.userMessage,
									assistantMessage: css?.assistantMessage
								}}
							/>
						{/each}
						{#if loading}
							<div class="flex items-center gap-2 text-tertiary">
								<Loader2 size={16} class="animate-spin" />
								<span class="text-sm">Processing...</span>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Input Container -->
			<div
				class={twMerge('border-t p-3', css?.inputContainer?.class, 'wm-chat-input')}
				style={css?.inputContainer?.style}
			>
				<ChatInput
					bind:value={inputValue}
					placeholder={resolvedConfig.placeholder}
					disabled={loading}
					onSend={handleSend}
					onKeydown={handleKeydown}
					customCss={{
						input: css?.input,
						button: css?.button
					}}
				/>
			</div>
		</div>
	{/if}
</RunnableWrapper>
