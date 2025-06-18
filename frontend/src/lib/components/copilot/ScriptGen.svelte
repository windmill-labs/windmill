<script lang="ts">
	import { run } from 'svelte/legacy'

	import { base } from '$lib/base'
	import { Button } from '../common'

	import { MAX_SCHEMA_LENGTH, SUPPORTED_LANGUAGES, addThousandsSeparator, copilot } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TooltipV2 from '$lib/components/meltComponents/Tooltip.svelte'
	import { dbSchemas, copilotInfo, type DBSchema, workspaceStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type SimpleEditor from '../SimpleEditor.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { writable } from 'svelte/store'
	import HighlightCode from '../HighlightCode.svelte'
	import LoadingIcon from '../apps/svelte-select/lib/LoadingIcon.svelte'
	import { sleep } from '$lib/utils'
	import { autoPlacement } from '@floating-ui/core'
	import { AlertTriangle, Ban, Check, ExternalLink, HistoryIcon, Wand2, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { isInitialCode } from '$lib/script_helpers'
	import { twMerge } from 'tailwind-merge'
	import { onDestroy } from 'svelte'
	import ProviderModelSelector from './chat/ProviderModelSelector.svelte'
	import { aiChatManager, AIMode } from './chat/AIChatManager.svelte'

	interface Props {
		// props
		iconOnly?: boolean
		lang: SupportedLanguage | 'bunnative' | 'frontend' | undefined
		editor: Editor | SimpleEditor | undefined
		diffEditor: DiffEditor | undefined
		inlineScript?: boolean
		args: Record<string, any>
		transformer?: boolean
		openAiChat?: boolean
	}

	let {
		iconOnly = false,
		lang = $bindable(),
		editor,
		diffEditor,
		inlineScript = false,
		args,
		transformer = false,
		openAiChat = false
	}: Props = $props()

	run(() => {
		if (lang == 'bunnative') {
			lang = 'bun'
		}
	})

	// state
	let funcDesc = $state('')
	let trimmedDesc = $state('')
	let genLoading: boolean = $state(false)
	let input: HTMLTextAreaElement | undefined = $state()
	let generatedCode = writable<string>('')
	let dbSchema: DBSchema | undefined = $state(undefined)
	let abortController: AbortController | undefined = $state(undefined)
	let blockPopupOpen = $state(false)
	let mode: 'gen' | 'edit' = $state('gen')

	let button: HTMLButtonElement | undefined = $state()

	run(() => {
		trimmedDesc = funcDesc.trim()
	})

	function determineModeFromEditor() {
		if (editor && isInitialCode(editor.getCode())) {
			mode = 'gen'
		} else {
			mode = 'edit'
		}
	}

	async function callCopilot() {
		if (mode === 'edit') {
			await copilot(
				{
					// @ts-ignore
					language: transformer && lang === 'frontend' ? 'transformer' : lang!,
					description: trimmedDesc,
					code: editor?.getCode() || '',
					dbSchema: dbSchema,
					type: 'edit',
					workspace: $workspaceStore!
				},
				generatedCode,
				abortController
			)
		} else {
			await copilot(
				{
					// @ts-ignore
					language: transformer && lang === 'frontend' ? 'transformer' : lang!,
					description: trimmedDesc,
					dbSchema: dbSchema,
					type: 'gen',
					workspace: $workspaceStore!
				},
				generatedCode,
				abortController
			)
		}
	}

	function handleAiButtonClick() {
		if (openAiChat) {
			determineModeFromEditor()
			openAiChatForScript()
		} else {
			determineModeFromEditor()
			setTimeout(() => {
				autoResize()
			}, 0)
		}
	}

	async function onGenerate(closePopup: () => void) {
		if (trimmedDesc.length <= 0) {
			return
		}
		savePrompt()
		try {
			genLoading = true
			blockPopupOpen = true
			abortController = new AbortController()
			await callCopilot()
			setupDiff()
			diffEditor?.setModified($generatedCode)
			blockPopupOpen = false
			await sleep(500)
			closePopup()
			await sleep(300)
			showDiff()
			funcDesc = ''
		} catch (err) {
			if (!abortController?.signal.aborted) {
				if (err?.message) {
					sendUserToast('Failed to generate code: ' + err.message, true)
				} else {
					sendUserToast('Failed to generate code', true)
					console.error(err)
				}
			}
		} finally {
			genLoading = false
			blockPopupOpen = false
			setTimeout(() => {
				autoResize()
			}, 0)
		}
	}

	function openAiChatForScript() {
		if (!openAiChat || !lang) {
			return
		}
		
		// Open the AI chat in script mode
		aiChatManager.openChat()
		aiChatManager.changeMode(AIMode.SCRIPT)
		
		// Set a default prompt if the user hasn't entered anything specific
		const defaultPrompt = mode === 'edit' 
			? 'Edit this script based on my requirements'
			: 'Generate a script that accomplishes the following task'
			
		aiChatManager.instructions = trimmedDesc.length > 0 ? trimmedDesc : defaultPrompt
		aiChatManager.sendRequest()
	}

	function acceptDiff() {
		editor?.setCode(diffEditor?.getModified() || '')
		editor?.format()
		clear()
	}

	function rejectDiff() {
		clear()
	}

	function setupDiff() {
		diffEditor?.setupModel(lang === 'frontend' ? 'javascript' : scriptLangToEditorLang(lang))
		diffEditor?.setOriginal(editor?.getCode() || '')
	}

	function showDiff() {
		diffEditor?.show()
		editor?.hide()
	}

	function hideDiff() {
		editor?.show()
		diffEditor?.hide()
	}

	run(() => {
		input && setTimeout(() => input?.focus(), 100)
	})

	function clear() {
		$generatedCode = ''
	}

	run(() => {
		lang && clear()
	})

	run(() => {
		!$generatedCode && hideDiff()
	})

	function updateSchema(lang, args, dbSchemas) {
		const schemaRes = lang === 'graphql' ? args.api : args.database
		if (typeof schemaRes === 'string') {
			const schemaPath = schemaRes.replace('$res:', '')
			if (schemaPath in dbSchemas && dbSchemas[schemaPath].lang === lang) {
				dbSchema = dbSchemas[schemaPath]
			} else {
				dbSchema = undefined
			}
		} else {
			dbSchema = undefined
		}
	}

	run(() => {
		updateSchema(lang, args, $dbSchemas)
	})

	let codeDiv: HTMLDivElement | undefined = $state()
	let lastScrollHeight = 0
	function updateScroll() {
		if (codeDiv && lastScrollHeight !== codeDiv.scrollHeight) {
			codeDiv.scrollTop = codeDiv.scrollHeight
			lastScrollHeight = codeDiv.scrollHeight
		}
	}

	let promptHistory: string[] = $state([])
	
	function getPromptStorageKey() {
		return 'prompts-' + lang
	}

	function safeLocalStorageOperation<T>(operation: () => T, defaultValue?: T): T | undefined {
		try {
			return operation()
		} catch (e) {
			console.error('error interacting with local storage', e)
			return defaultValue
		}
	}

	function getPromptHistory() {
		const storageKey = getPromptStorageKey()
		promptHistory = safeLocalStorageOperation(
			() => JSON.parse(localStorage.getItem(storageKey) || '[]'),
			[]
		) || []
	}

	function savePrompt() {
		if (promptHistory.includes(trimmedDesc)) {
			return
		}
		promptHistory.unshift(trimmedDesc)
		while (promptHistory.length > 5) {
			promptHistory.pop()
		}
		const storageKey = getPromptStorageKey()
		safeLocalStorageOperation(() => 
			localStorage.setItem(storageKey, JSON.stringify(promptHistory))
		)
	}

	function clearPromptHistory() {
		promptHistory = []
		const storageKey = getPromptStorageKey()
		safeLocalStorageOperation(() => 
			localStorage.setItem(storageKey, JSON.stringify(promptHistory))
		)
	}
	run(() => {
		lang && getPromptHistory()
	})

	run(() => {
		$generatedCode && updateScroll()
	})

	let innerWidth = $state(0)

	function autoResize() {
		if (input) {
			const maxLinesHeight = innerWidth > 2500 ? 146 : 130 // Adjust this value based on your font size and line-height to fit 5 lines
			input.style.height = 'auto' // Reset height to recalibrate
			const newHeight = Math.min(input.scrollHeight + 2, maxLinesHeight) // Calculate new height, but not exceed max
			input.style.height = newHeight + 'px' // Set new height
			if (input.scrollHeight + 2 > maxLinesHeight) {
				input.scrollTop = input.scrollHeight
				input.style.overflowY = 'scroll'
			} else {
				input.style.overflowY = 'hidden'
			}
		}
	}

	function handlePublicOnlySelected({ detail }: { detail: string }) {
		if (!dbSchema) return
		;(dbSchema as any).publicOnly = detail === 'true'
	}

	onDestroy(() => {
		abortController?.abort()
	})
</script>

<svelte:window onresize={autoResize} bind:innerWidth />

{#if genLoading}
	<div transition:fade class="fixed z-[4999] inset-0 bg-gray-500/75"></div>
{/if}

{#if $generatedCode.length > 0 && !genLoading}
	{#if inlineScript}
		<div class="flex gap-1">
			<Button
				title="Discard generated code"
				btnClasses="!font-medium px-2 w-7"
				size="xs"
				color="red"
				on:click={rejectDiff}
				variant="contained"
				startIcon={{ icon: X }}
				iconOnly
			/>
			<Button
				title="Accept generated code"
				btnClasses="!font-medium px-2 w-7"
				size="xs"
				color="green"
				on:click={acceptDiff}
				iconOnly
				startIcon={{ icon: Check }}
			/>
		</div>
	{:else}
		<div class="flex gap-1 px-2">
			<Button
				title="Discard generated code"
				btnClasses="!font-medium px-2"
				size="xs"
				color="red"
				on:click={rejectDiff}
				variant="contained"
				startIcon={{ icon: X }}
				{iconOnly}
			>
				Discard
			</Button><Button
				title="Accept generated code"
				btnClasses="!font-medium px-2"
				size="xs"
				color="green"
				on:click={acceptDiff}
				startIcon={{ icon: Check }}
				{iconOnly}
			>
				Accept
			</Button>
		</div>
	{/if}
{/if}
{#if ($generatedCode.length === 0 || genLoading) && SUPPORTED_LANGUAGES.has(lang ?? '')}
	<Popover
		floatingConfig={{
			middleware: [
				autoPlacement({
					allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
				})
			]
		}}
		disabled={blockPopupOpen}
	>
		{#snippet trigger()}
			{#if inlineScript}
				<Button
					size="xs"
					color={genLoading ? 'red' : 'light'}
					btnClasses={genLoading ? '!px-3 z-[5000]' : '!px-2'}
					propagateEvent={!genLoading}
					on:click={genLoading
						? () => abortController?.abort()
						: handleAiButtonClick}
					bind:element={button}
					iconOnly
					title="Generate code from prompt"
					startIcon={genLoading
						? { icon: Ban }
						: { icon: Wand2, classes: 'text-violet-800 dark:text-violet-400' }}
				/>
			{:else}
				<Button
					title="Generate code from prompt"
					btnClasses={twMerge(
						'!font-medium',
						genLoading ? 'z-[5000]' : 'text-violet-800 dark:text-violet-400'
					)}
					size="xs"
					color={genLoading ? 'red' : 'light'}
					spacingSize="md"
					startIcon={genLoading ? { icon: Ban } : { icon: Wand2 }}
					propagateEvent={!genLoading}
					on:click={genLoading
						? () => abortController?.abort()
						: handleAiButtonClick}
					bind:element={button}
					{iconOnly}
				>
					{#if genLoading}
						Stop
					{:else}
						AI Gen
					{/if}
				</Button>
			{/if}
		{/snippet}
		{#snippet content({ close })}
			<div class="p-4">
				{#if genLoading}
					<div class="w-[42rem] min-h-[3rem] max-h-[34rem] overflow-y-scroll" bind:this={codeDiv}>
						{#if $generatedCode.length > 0}
							<div class="overflow-x-scroll">
								<HighlightCode language={lang} code={$generatedCode} /></div
							>
						{:else}
							<LoadingIcon />
						{/if}
					</div>
				{:else if $copilotInfo.enabled}
					<div class="flex flex-col gap-4">
						<div class="flex flex-row justify-between items-center w-96 gap-2">
							<ToggleButtonGroup class="w-auto shrink-0 h-auto" bind:selected={mode}>
								{#snippet children({ item })}
									<ToggleButton
										value={'gen'}
										label="Generate from scratch"
										light
										class="px-2"
										{item}
									/>
									<ToggleButton
										value={'edit'}
										label="Edit existing code"
										light
										class="px-2"
										{item}
									/>
								{/snippet}
							</ToggleButtonGroup>

							<ProviderModelSelector />
						</div>
						<div class="flex w-96 items-start">
							<textarea
								bind:this={input}
								bind:value={funcDesc}
								oninput={autoResize}
								onkeydown={({ key, shiftKey }) => {
									if (key === 'Enter' && !shiftKey && trimmedDesc.length > 0) {
										onGenerate(() => close())
										return false
									}
								}}
								placeholder={mode === 'edit'
									? 'Describe the changes you want'
									: 'Describe what the script should do'}
								rows="1"
								class="resize-none overflow-hidden"
							></textarea>
							<Button
								size="xs"
								color="light"
								buttonType="button"
								btnClasses="!h-[34px] qhd:!h-[38px] !ml-2 text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
								title="Generate code from prompt"
								aria-label="Generate"
								on:click={() => {
									onGenerate(() => close())
								}}
								disabled={trimmedDesc.length <= 0}
								iconOnly
								startIcon={{ icon: Wand2 }}
							/>
						</div>
						{#if promptHistory.length > 0}
							<div class="w-96 flex flex-col gap-1">
								{#each promptHistory as p}
									<Button
										size="xs2"
										color="light"
										btnClasses="justify-start overflow-x-scroll no-scrollbar"
										startIcon={{ icon: HistoryIcon, classes: 'shrink-0' }}
										on:click={() => {
											funcDesc = p
											setTimeout(() => {
												autoResize()
											}, 0)
										}}>{p}</Button
									>
								{/each}
								<button
									class="underline text-xs text-start px-2 text-secondary font-normal"
									onclick={clearPromptHistory}>clear history</button
								>
							</div>
						{/if}

						{#if ['postgresql', 'mysql', 'snowflake', 'bigquery', 'mssql', 'graphql, oracledb'].includes(lang ?? '') && dbSchema?.lang === lang}
							<div class="flex flex-row items-center justify-between gap-2 w-96">
								<div class="flex flex-row items-center gap-1">
									<p class="text-xs text-secondary">
										Context: {lang === 'graphql' ? 'GraphQL' : 'DB'} schema
									</p>
									<Tooltip placement="top">
										We pass the selected schema to GPT-4 Turbo for better script generation.
									</Tooltip>
									{#if dbSchema && dbSchema.stringified.length > MAX_SCHEMA_LENGTH}
										<TooltipV2 notClickable placement="top">
											<AlertTriangle size={16} class="text-yellow-500" />
											{#snippet text()}
												The schema is about {addThousandsSeparator(
													(dbSchema?.stringified.length ?? 0) / 3.5
												)}
												tokens. To avoid exceeding the model's context length, it will be truncated to
												{addThousandsSeparator(MAX_SCHEMA_LENGTH / 3.5)}
												tokens.
											{/snippet}
										</TooltipV2>
									{/if}
								</div>
								{#if dbSchema && dbSchema.lang !== 'graphql' && (dbSchema.schema?.public || dbSchema.schema?.PUBLIC || dbSchema.schema?.dbo)}
									<ToggleButtonGroup
										class="w-auto shrink-0"
										selected={dbSchema?.publicOnly ? 'true' : 'false'}
										on:selected={handlePublicOnlySelected}
									>
										{#snippet children({ item })}
											<ToggleButton
												value={'true'}
												label={(dbSchema?.lang !== 'graphql' && dbSchema?.schema?.dbo
													? 'Dbo'
													: 'Public') + ' schema'}
												small
												light
												{item}
											/>
											<ToggleButton value={'false'} label="All schemas" small light {item} />
										{/snippet}
									</ToggleButtonGroup>
								{/if}
							</div>
						{/if}
					</div>
				{:else}
					<p class="text-sm">
						Enable Windmill AI in the <a
							href="{base}/workspace_settings?tab=ai"
							target="_blank"
							class="inline-flex flex-row items-center gap-1"
						>
							workspace settings <ExternalLink size={16} />
						</a>
					</p>
				{/if}
			</div>
		{/snippet}
	</Popover>
{/if}
