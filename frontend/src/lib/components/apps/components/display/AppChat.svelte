<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
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
	import { Button } from '$lib/components/common'
	import { Send } from 'lucide-svelte'
	import { initCss } from '../../utils'

	interface Message {
		role: 'user' | 'assistant'
		content: string
		timestamp: number
		streaming?: boolean
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
	let messages: Message[] = $state([])
	let inputValue = $state('')
	let messagesContainer: HTMLDivElement | undefined = $state()

	// Streaming state management
	let currentStreamingMessageIndex: number | undefined = $state(undefined)
	let accumulatedContent = $state('')

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

	// Parse streaming tokens (similar to FlowChatManager parseStreamDeltas)
	function parseStreamDeltas(streamData: string): string {
		const lines = streamData.trim().split('\n')
		let content = ''

		for (const line of lines) {
			if (!line.trim()) continue
			try {
				const parsed = JSON.parse(line)
				// Accumulate content from token_delta events
				if (parsed.type === 'token_delta' && parsed.content) {
					content += parsed.content
				}
			} catch (e) {
				console.error('Failed to parse stream line:', line, e)
			}
		}

		return content
	}

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
	{componentInput}
	{id}
	{recomputeIds}
	{outputs}
	doOnSuccess={resolvedConfig.onSuccess}
	doOnError={resolvedConfig.onError}
	{errorHandledByComponent}
	autoRefresh={false}
	{render}
	onSuccess={(result) => {
		if (result !== undefined && result !== null) {
			// Parse the result to extract streaming content
			let newContent = ''

			if (typeof result === 'string') {
				// Try to parse as streaming format
				const parsedContent = parseStreamDeltas(result)
				if (parsedContent) {
					newContent = parsedContent
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
					idx === currentStreamingMessageIndex
						? { ...msg, content: accumulatedContent, streaming: loading }
						: msg
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
					timestamp: Date.now(),
					streaming: loading
				}
				messages = [...messages, assistantMessage]

				// If still loading, track this message for updates
				if (loading) {
					currentStreamingMessageIndex = messages.length - 1
				}
			}
		}
	}}
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
					'flex-1 overflow-y-auto p-4 space-y-3',
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
					{#each messages as message (message.timestamp)}
						<div class={twMerge('flex', message.role === 'user' ? 'justify-end' : 'justify-start')}>
							<div
								class={twMerge(
									'max-w-[80%] rounded-lg px-4 py-2 break-words',
									message.role === 'user'
										? twMerge(
												'bg-blue-600 text-white',
												css?.userMessage?.class,
												'wm-chat-user-message'
											)
										: twMerge(
												'bg-surface-secondary text-primary',
												css?.assistantMessage?.class,
												'wm-chat-assistant-message'
											)
								)}
								style={message.role === 'user'
									? css?.userMessage?.style
									: css?.assistantMessage?.style}
							>
								<div class="whitespace-pre-wrap">
									{message.content}
									{#if message.streaming}
										<span class="inline-block w-1 h-4 ml-1 bg-current animate-pulse">▌</span>
									{/if}
								</div>
							</div>
						</div>
					{/each}
				{/if}
			</div>

			<!-- Input Container -->
			<div class="border-t p-3">
				<div class="flex gap-2">
					<input
						type="text"
						bind:value={inputValue}
						placeholder={resolvedConfig.placeholder}
						disabled={loading}
						onkeydown={handleKeydown}
						onpointerdown={stopPropagation(bubble('pointerdown'))}
						class={twMerge(
							'flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed',
							css?.input?.class,
							'wm-chat-input'
						)}
						style={css?.input?.style}
					/>
					<Button
						on:click={handleSend}
						disabled={loading || !inputValue.trim()}
						size="sm"
						color="blue"
						btnClasses={twMerge(css?.button?.class, 'wm-chat-send-button')}
						style={twMerge('min-width: 80px;', css?.button?.style)}
						{loading}
					>
						{#if loading}
							Sending...
						{:else}
							<div class="flex items-center gap-2">
								<Send size={16} />
								<span>Send</span>
							</div>
						{/if}
					</Button>
				</div>
			</div>
		</div>
	{/if}
</RunnableWrapper>
