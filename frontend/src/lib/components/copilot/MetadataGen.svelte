<script lang="ts">
	import { getCompletion, getResponseFromEvent } from './lib'
	import { isInitialCode } from '$lib/script_helpers'
	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import { copilotInfo, metadataCompletionEnabled } from '$lib/stores'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import autosize from '$lib/autosize'
	import type { FlowValue } from '$lib/gen'
	import { yamlStringifyExceptKeys } from './utils'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

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
	} = {
		summary: {
			system: `
You are a helpful AI assistant. You generate very brief summaries from scripts.
The summaries need to be as short as possible (maximum 8 words) and only give a global idea. Do not specify the programming language. Do not use any punctation. Avoid using prepositions and articles.
Examples: List the commits of a GitHub repository, Divide a number by 16, etc..
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
`,
			user: `
Generate a description for the flow below:
{flow}`,
			placeholderName: 'flow'
		}
	}

	export let content: string | undefined
	export let code: string | undefined = undefined
	export let flow: FlowValue | undefined = undefined
	export let promptConfigName: keyof typeof promptConfigs
	export let generateOnAppear: boolean = false
	export let elementType: 'input' | 'textarea' = 'input'
	export let elementProps: Record<string, any> = {}

	let el: HTMLElement | undefined
	let generatedContent = ''
	let active = false
	let loading = false
	let abortController = new AbortController()
	let manualDisabled = false
	let width = 0
	let genHeight = 0

	let focused = false
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

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	$: content && dispatchIfMounted('change', { content })

	$: active =
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

	$: focused && (manualDisabled = false)

	$: if (content) {
		abortController.abort()
		generatedContent = ''
	} else {
		manualDisabled = false
	}

	$: if (elementType === 'textarea' && el !== undefined && !content) {
		el.style.height = Math.max(genHeight + 34, 58) + 'px'
	}

	onDestroy(() => {
		abortController.abort()
	})
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={twMerge('relative', $$props.class)}
	bind:clientWidth={width}
	on:keydown={(event) => {
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
		class="absolute left-[0.5rem] {elementType === 'textarea'
			? 'top-[1.3rem]'
			: 'top-[0.3rem]'}  flex flex-row gap-2 items-start pointer-events-none"
	>
		{#if active}
			<span
				class={twMerge(
					'absolute text-xs bg-violet-100 text-violet-800 dark:bg-gray-700 dark:text-violet-400 px-1 py-0.5 rounded-md flex flex-row items-center justify-center gap-2 transition-all shrink-0',
					!loading && generatedContent.length > 0
						? 'bg-green-100 text-green-800 dark:text-green-400 dark:bg-green-700'
						: ''
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
					'text-sm leading-6 indent-[3.5rem] text-gray-500 dark:text-gray-400 pr-1',
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
			<div class="flex flex-row-reverse !text-3xs text-tertiary -mt-4">GH Markdown</div>
			<textarea
				bind:this={el}
				bind:value={content}
				use:autosize
				{...elementProps}
				placeholder={!active ? elementProps.placeholder : ''}
				class={active ? '!indent-[3.5rem]' : ''}
				on:focus={() => (focused = true)}
				on:blur={() => (focused = false)}
			></textarea>
		</div>
	{:else}
		<input
			bind:this={el}
			bind:value={content}
			placeholder={!active ? elementProps.placeholder : ''}
			class={active ? '!indent-[3.5rem]' : ''}
			on:focus={() => (focused = true)}
			on:blur={() => (focused = false)}
		/>
	{/if}
	<!-- <slot {updateFocus} {active} {generatedContent} classNames={active ? '!indent-[8.8rem]' : ''} /> -->
</div>
