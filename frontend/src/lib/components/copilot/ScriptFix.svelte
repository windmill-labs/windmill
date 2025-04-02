<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, copilot } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { dbSchemas, copilotInfo, type DBSchema, workspaceStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { writable } from 'svelte/store'
	import { WindmillIcon } from '../icons'
	import HighlightCode from '../HighlightCode.svelte'
	import LoadingIcon from '../apps/svelte-select/lib/LoadingIcon.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { Check, Wand2, X, RotateCw } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	// props
	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined
	export let error: string
	export let args: Record<string, any>
	export let chatMode: boolean = false

	// state
	let genLoading: boolean = false
	let generatedCode = writable<string>('')
	let generatedExplanation = writable<string>('')
	let dbSchema: DBSchema | undefined = undefined
	let abortController: AbortController | undefined = undefined

	const dispatch = createEventDispatcher<{
		fix: null
	}>()

	async function onFix() {
		if (!error) {
			return
		}
		try {
			genLoading = true
			abortController = new AbortController()
			await copilot(
				{
					language: lang,
					code: editor?.getCode() || '',
					error,
					dbSchema: dbSchema,
					type: 'fix',
					workspace: $workspaceStore!
				},
				generatedCode,
				abortController,
				generatedExplanation
			)
			setupDiff()
			diffEditor?.setModified($generatedCode)
			showDiff()
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
		}
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
		diffEditor?.setupModel(scriptLangToEditorLang(lang))
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

	function clear() {
		$generatedCode = ''
		$generatedExplanation = ''
	}

	$: lang && clear()

	$: !$generatedCode && hideDiff()

	function updateSchema(lang, args) {
		const schemaRes = lang === 'graphql' ? args.api : args.database
		if (typeof schemaRes === 'string') {
			dbSchema = $dbSchemas[schemaRes.replace('$res:', '')]
		}
	}
	$: updateSchema(lang, args)

	let popover: Popover | undefined = undefined
</script>

{#if SUPPORTED_LANGUAGES.has(lang)}
	{#if !genLoading && $generatedCode.length > 0}
		<div class="flex gap-1">
			<Button
				title="Discard generated code"
				size="xs"
				color="red"
				spacingSize="xs2"
				on:click={() => {
					popover?.close()
					rejectDiff()
				}}
				variant="contained"
				startIcon={{ icon: X }}
				propagateEvent={true}
			>
				Discard
			</Button>
			<Button
				title="Accept generated code"
				size="xs"
				color="green"
				spacingSize="xs2"
				on:click={() => {
					popover?.close()
					acceptDiff()
				}}
				startIcon={{ icon: Check }}
				propagateEvent={true}
			>
				Accept
			</Button>
			<Button
				size="xs"
				color="light"
				spacingSize="xs2"
				on:click={() => {
					$generatedExplanation = ''
					popover?.open()
					onFix()
				}}
				startIcon={{ icon: RotateCw }}
				title="Retry"
				btnClasses="text-violet-800 dark:text-violet-400"
			/>
		</div>
	{/if}
	<Popover
		bind:this={popover}
		floatingConfig={{
			middleware: [
				autoPlacement({
					allowedPlacements: ['bottom-end', 'top-end']
				})
			]
		}}
		closeOnOutsideClick={!genLoading}
		closeButton={!genLoading}
		displayArrow={true}
	>
		<svelte:fragment slot="trigger" let:isOpen>
			<div class="flex flex-row">
				<Button
					title="Fix code"
					size="xs"
					color={genLoading ? 'red' : 'light'}
					spacingSize="xs2"
					startIcon={genLoading ? undefined : { icon: Wand2 }}
					propagateEvent={!chatMode}
					on:click={chatMode
						? () => dispatch('fix')
						: genLoading
						? () => abortController?.abort()
						: $generatedCode.length > 0
							? () => {}
							: () => onFix()}
					btnClasses={genLoading
						? ''
						: 'text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700 min-w-[84px]'}
				>
					{#if genLoading}
						<WindmillIcon
							white
							class="mr-1 text-white"
							height="16px"
							width="20px"
							spin="veryfast"
						/>
						Stop
					{:else if $generatedCode.length > 0}
						<span class="text-xs">{isOpen ? 'Hide' : 'Show'}</span>
					{:else}
						AI Fix
					{/if}
				</Button>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content">
			<div class="p-4">
				{#if $copilotInfo.enabled}
					<div class="w-[42rem] min-h-[3rem] max-h-[34rem] overflow-y-auto">
						{#if $generatedCode.length > 0 && genLoading}
							<div class="overflow-x-scroll">
								<HighlightCode language={lang} code={$generatedCode} />
							</div>
						{:else if genLoading}
							<LoadingIcon />
						{/if}
						{#if $generatedExplanation.length > 0}
							<p class="text-sm mt-2"
								><span class="font-bold">Explanation: test</span> {$generatedExplanation}</p
							>
						{/if}
					</div>
				{:else}
					<div class="w-80">
						<p class="text-sm"
							>Enable Windmill AI in the <a
								class="inline-flex flex-row items-center gap-1"
								href="{base}/workspace_settings?tab=ai"
								target="_blank">workspace settings</a
							></p
						></div
					>
				{/if}
			</div>
		</svelte:fragment>
	</Popover>
{/if}
