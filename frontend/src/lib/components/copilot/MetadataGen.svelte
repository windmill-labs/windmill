<script lang="ts">
	import { getCompletion, getResponseFromEvent } from './lib'
	import { isInitialCode } from '$lib/script_helpers'
	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import { metadataCompletionEnabled } from '$lib/stores'
	import { copilotInfo } from '$lib/aiStore'
	import { onDestroy } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import autosize from '$lib/autosize'
	import type { FlowValue } from '$lib/gen'
	import { yamlStringifyExceptKeys } from './utils'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { validateToolName } from '$lib/components/graph/renderers/nodes/AIToolNode.svelte'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '../text_input/TextInput.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'

	type PromptConfig = {
		system: string
		user: string
		placeholderName: 'code' | 'flow'
	}

	const promptConfigs: {
		summary: PromptConfig
		description: PromptConfig
		flowSummary: PromptConfig
		flowDescription: PromptConfig
		agentToolFunctionName: PromptConfig
	} = {
		summary: {
			system: `
You are a helpful AI assistant. You generate very brief summaries from scripts.
The summaries need to be as short as possible (maximum 8 words) and only give a global idea. Do not specify the programming language. Do not use any punctation. Avoid using prepositions and articles.
Examples: List the commits of a GitHub repository, Divide a number by 16, etc..
**Return only the summary, no other text.**
`,
			user: `
Generate a very short summary for the script below:
\'\'\'code
{code}
\`\`\`
`,
			placeholderName: 'code'
		},
		description: {
			system: `
You are a helpful AI assistant. You generate descriptions from scripts.
These descriptions are used to explain to other users what the script does and how to use it.
Be as short as possible to give a global idea, maximum 3-4 sentences.
All scripts export an asynchronous function called main, do not include it in the description.
Do not describe how to call it either.
**Return only the description, no other text.**
`,
			user: `
Generate a description for the script below:
\'\'\'code
{code}
\`\`\`
`,
			placeholderName: 'code'
		},
		flowSummary: {
			system: `
			You are a helpful AI assistant. You generate very brief summaries from scripts.
The summaries need to be as short as possible (maximum 8 words) and only give a global idea. Do not use any punctation. Avoid using prepositions and articles.
**Return only the summary, no other text.**
`,
			user: `
Summarize the flow below in one very short sentence without punctation:
{flow}`,
			placeholderName: 'flow'
		},
		flowDescription: {
			system: `
You are a helpful AI assistant. You generate descriptions from flow.
These descriptions are used to explain to other users what the flow does and how to use it.
Be as short as possible to give a global idea, maximum 3-4 sentences.
Do not include line breaks.
**Return only the description, no other text.**
`,
			user: `
Generate a description for the flow below:
{flow}`,
			placeholderName: 'flow'
		},
		agentToolFunctionName: {
			system: `
You are a helpful AI assistant. You generate tool names from scripts.
These tool names will be used by an AI agent to call this tool.
It has to be based on the script code content not on the main function name.
It has to respect the following regex: /[a-zA-Z0-9_]+/
Examples: generate_image, classify_image, summarize_text, etc.
**Return only the tool name, no other text.**
`,
			user: `
Generate a tool name for the script below:
{code}`,
			placeholderName: 'code'
		}
	}

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		content: string | undefined
		code?: string | undefined
		flow?: FlowValue | undefined
		promptConfigName: keyof typeof promptConfigs
		generateOnAppear?: boolean
		elementType?: 'input' | 'textarea'
		elementProps?: Record<string, any>
		class?: string
		onChange?: (content: string) => void
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		content = $bindable(),
		code = undefined,
		flow = undefined,
		promptConfigName,
		generateOnAppear = false,
		elementType = 'input',
		elementProps = {},
		class: clazz = '',
		onChange = undefined
	}: Props = $props()

	let el: HTMLElement | undefined = $state()
	let generatedContent = $state('')
	let active = $state(false)
	let loading = $state(false)
	let abortController = $state(new AbortController())
	let manualDisabled = $state(false)
	let width = $state(0)
	let genHeight = $state(0)

	let focused = $state(false)
	let config: PromptConfig = promptConfigs[promptConfigName]

	async function generateContent(automatic = false) {
		abortController = new AbortController()
		loading = true
		try {
			const placeholderContent = flow ? yamlStringifyExceptKeys(flow, ['lock']) : code

			if (!placeholderContent) {
				sendUserToast('Could not generate summary: no content to generate from', true)
				return
			}
			// return
			const messages: ChatCompletionMessageParam[] = [
				{
					role: 'system',
					content: config.system
				},
				{
					role: 'user',
					content: config.user.replace(`{${config.placeholderName}}`, placeholderContent)
				}
			]
			const response = await getCompletion(messages, abortController)
			generatedContent = ''
			for await (const chunk of response) {
				generatedContent += getResponseFromEvent(chunk)
			}
		} catch (err) {
			if (!abortController.signal.aborted) {
				if (automatic) {
					console.error('Could not generate summary ' + err)
				} else {
					sendUserToast('Could not generate summary: ' + err, true)
				}
			}
		} finally {
			loading = false
		}
	}

	if (
		$copilotInfo.enabled &&
		$metadataCompletionEnabled &&
		generateOnAppear &&
		!content &&
		code &&
		!isInitialCode(code)
	) {
		setTimeout(() => {
			el?.focus()
		}, 0)
		generateContent(true)
	}

	$effect(() => {
		content && onChange?.(content)
	})

	$effect(() => {
		active =
			$copilotInfo.enabled &&
			$metadataCompletionEnabled &&
			!content &&
			(loading || focused || !!generatedContent) &&
			!manualDisabled &&
			((config.placeholderName === 'code' && !!code) ||
				(config.placeholderName === 'flow' &&
					!!flow &&
					Array.isArray(flow.modules) &&
					flow.modules.length > 0))
	})

	$effect(() => {
		focused && (manualDisabled = false)
	})

	$effect(() => {
		if (content) {
			abortController.abort()
			generatedContent = ''
		} else {
			manualDisabled = false
		}
	})

	$effect(() => {
		if (elementType === 'textarea' && el !== undefined && !content) {
			el.style.height = Math.max(genHeight + 34, 58) + 'px'
		}
	})

	onDestroy(() => {
		abortController.abort()
	})
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge('relative', clazz)}
	bind:clientWidth={width}
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: (value) => {
			if (value) {
				content = value
				onChange?.(content)
			}
		}
	}}
	onkeydown={(event) => {
		if (!$copilotInfo.enabled || !$metadataCompletionEnabled) {
			return
		}
		if (event.key === 'Tab') {
			if (manualDisabled) {
				event.preventDefault()
				manualDisabled = false
			} else if (!loading && generatedContent) {
				event.preventDefault()
				content = generatedContent
				generatedContent = ''
			} else if (!loading && !content) {
				event.preventDefault()
				generateContent()
			}
		} else if (event.key === 'Escape' && !manualDisabled) {
			event.preventDefault()
			event.stopPropagation()
			if (loading) {
				abortController.abort()
			} else {
				manualDisabled = true
				generatedContent = ''
			}
		} else if (event.key === 'Backspace' && !loading && !content) {
			manualDisabled = true
			generatedContent = ''
		}
	}}
>
	<div
		class="absolute left-3 {elementType === 'textarea'
			? 'top-[1.3rem]'
			: 'top-[0.3rem]'}  flex flex-row gap-2 items-start pointer-events-none"
	>
		{#if active}
			<span
				class={twMerge(
					'rounded-md px-1',
					AIBtnClasses(!loading && generatedContent.length > 0 ? 'green' : 'selected')
				)}
			>
				<span class="px-0.5 py-0.5 rounded-md text-2xs text-bold flex flex-row items-center gap-1">
					{#if loading}
						ESC
					{:else}
						TAB
					{/if}
					{#if loading}
						<Loader2 class="animate-spin" size={12} />
					{:else if generatedContent}
						<Check size={12} />
					{:else}
						<Wand2 size={12} />
					{/if}
				</span>
			</span>
			<div
				bind:clientHeight={genHeight}
				class={twMerge(
					'text-sm leading-6 indent-0 text-gray-500 dark:text-gray-400 pr-1',
					elementType === 'input' ? 'text-ellipsis overflow-hidden whitespace-nowrap' : ''
				)}
				style={elementType === 'input' ? `max-width: calc(${width}px - 0.5rem)` : ''}
			>
				{generatedContent}
			</div>
		{/if}
	</div>
	{#if elementType === 'textarea'}
		<div>
			<div class="flex flex-row-reverse !text-3xs text-primary -mt-4">GH Markdown</div>
			<textarea
				bind:this={el}
				bind:value={content}
				use:autosize
				{...elementProps}
				placeholder={!active ? elementProps.placeholder : ''}
				class="{inputBaseClass} {inputSizeClasses.md} {inputBorderClass()} w-full"
				onfocus={() => (focused = true)}
				onblur={() => (focused = false)}
			></textarea>
		</div>
	{:else}
		<input
			bind:this={el}
			bind:value={content}
			placeholder={!active ? elementProps.placeholder : ''}
			class={twMerge(
				inputBaseClass,
				inputSizeClasses.md,
				inputBorderClass({
					error: promptConfigName === 'agentToolFunctionName' && !validateToolName(content ?? '')
				}),
				'w-full'
			)}
			onfocus={() => (focused = true)}
			onblur={() => (focused = false)}
		/>
		{#if promptConfigName === 'agentToolFunctionName' && !validateToolName(content ?? '')}
			<p class="text-3xs text-red-400 leading-tight mt-0.5">
				Invalid tool name, should only contain letters, numbers and underscores
			</p>
		{/if}
	{/if}
	<!-- <slot {updateFocus} {active} {generatedContent} classNames={active ? '!indent-[8.8rem]' : ''} /> -->
</div>
