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

	interface Message {
		role: 'user' | 'assistant'
		content: string
		timestamp: number
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
		messages: [] as Message[]
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
	let lastProcessedResult: any = $state(undefined)

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

	// Watch result changes for streaming updates
	$effect(() => {
		// Only process if result changed and we have a result
		if (result !== lastProcessedResult && result !== undefined && result !== null) {
			lastProcessedResult = result

			// Parse the result to extract streaming content
			let newContent = ''

			if (typeof result === 'string') {
				// Try to parse as streaming format
				const parsed = parseStreamDeltas(result)
				if (parsed.content) {
					newContent = parsed.content
					accumulatedContent += newContent
				} else {
					// If not streaming format, use the whole string
					accumulatedContent = result
				}
			} else {
				// Non-string result (final result or error)
				// Check if result has an "output" key and use that instead
				if (result && typeof result === 'object' && 'output' in result) {
					accumulatedContent =
						typeof result.output === 'string'
							? result.output
							: JSON.stringify(result.output, null, 2)
				} else {
					accumulatedContent = JSON.stringify(result, null, 2)
				}
			}

			// If we have a streaming message, update it
			if (currentStreamingMessageIndex !== undefined) {
				messages = messages.map((msg, idx) =>
					idx === currentStreamingMessageIndex ? { ...msg, content: accumulatedContent } : msg
				)

				// If loading is done, finalize the message
				if (!loading) {
					currentStreamingMessageIndex = undefined
					accumulatedContent = ''
				}
			}
			// Create a new assistant message only if we have content
			else if (accumulatedContent.length > 0) {
				const assistantMessage: Message = {
					role: 'assistant',
					content: accumulatedContent,
					timestamp: Date.now()
				}
				messages = [...messages, assistantMessage]

				// If still loading, track this message for updates
				if (loading) {
					currentStreamingMessageIndex = messages.length - 1
				}
			}
		}
	})

	// Handle send message
	async function handleSend() {
		if (!inputValue.trim() || loading) return

		const userMessage = inputValue.trim()
		inputValue = ''

		// Add user message to chat
		const newUserMessage: Message = {
			role: 'user',
			content: userMessage,
			timestamp: Date.now()
		}
		messages = [...messages, newUserMessage]

		// Reset streaming state for new message
		currentStreamingMessageIndex = undefined
		accumulatedContent = ''

		// Trigger the runnable with the message as input
		if (!runnableComponent) {
			runnableWrapper?.handleSideEffect(true)
		} else {
			await runnableComponent?.runComponent(true, undefined, undefined, {
				user_message: userMessage
			})
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
	{errorHandledByComponent}
	autoRefresh={false}
	{render}
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
						{#each messages as message (message.timestamp)}
							<ChatMessage
								role={message.role}
								content={message.content}
								enableMarkdown={false}
								enableS3Display={false}
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
			<div class="border-t p-3">
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
