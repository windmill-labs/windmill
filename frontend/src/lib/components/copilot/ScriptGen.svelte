<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, copilot } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import {
		faCancel,
		faCheck,
		faClose,
		faMagicWandSparkles
	} from '@fortawesome/free-solid-svg-icons'
	import Popup from '../common/popup/Popup.svelte'
	import { Icon } from 'svelte-awesome'
	import { dbSchemas, existsOpenaiResourcePath, type DBSchema } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Selection } from 'monaco-editor/esm/vs/editor/editor.api'
	import type SimpleEditor from '../SimpleEditor.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { writable } from 'svelte/store'
	import { WindmillIcon } from '../icons'
	import HighlightCode from '../HighlightCode.svelte'
	import LoadingIcon from '../apps/svelte-select/lib/LoadingIcon.svelte'
	import { sleep } from '$lib/utils'
	import { autoPlacement } from '@floating-ui/core'

	// props
	export let iconOnly: boolean = false
	export let lang: SupportedLanguage | 'frontend'
	export let editor: Editor | SimpleEditor | undefined
	export let diffEditor: DiffEditor | undefined
	export let inlineScript = false

	// state
	let funcDesc: string = ''
	let genLoading: boolean = false
	let input: HTMLInputElement | undefined
	let generatedCode = writable<string>('')
	let selection: Selection | undefined
	let isEdit = false
	let dbSchema: DBSchema | undefined = undefined
	let abortController: AbortController | undefined = undefined

	let button: HTMLButtonElement | undefined

	async function onGenerate(closePopup: () => void) {
		if (funcDesc.length <= 0) {
			return
		}
		try {
			genLoading = true
			abortController = new AbortController()
			if (isEdit && selection) {
				const selectedCode = editor?.getSelectedLines() || ''
				const originalCode = editor?.getCode() || ''
				await copilot(
					{
						language: lang,
						description: funcDesc,
						code: selectedCode,
						dbSchema: dbSchema,
						type: 'edit'
					},
					generatedCode,
					abortController
				)
				setupDiff()
				diffEditor?.setModified(originalCode.replace(selectedCode, $generatedCode + '\n'))
			} else {
				await copilot(
					{
						language: lang,
						description: funcDesc,
						dbSchema: dbSchema,
						type: 'gen'
					},
					generatedCode,
					abortController
				)
				setupDiff()
				diffEditor?.setModified($generatedCode)
			}
			await sleep(500)
			closePopup()
			await sleep(300)
			showDiff()
			funcDesc = ''
		} catch (err) {
			if (err?.message) {
				sendUserToast('Failed to generate code: ' + err.message, true)
			} else {
				sendUserToast('Failed to generate code', true)
				console.error(err)
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

	function setSelectionHandler() {
		editor?.onDidChangeCursorSelection((e) => {
			selection = e.selection
		})
	}

	$: input?.focus()

	function clear() {
		$generatedCode = ''
	}

	$: lang && clear()

	$: !$generatedCode && hideDiff()
	$: editor && setSelectionHandler()
	$: selection && (isEdit = !selection.isEmpty())
	$: dbSchema = $dbSchemas[Object.keys($dbSchemas)[0]]
</script>

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
			>
				<Icon data={faClose} />
			</Button><Button
				title="Accept generated code"
				btnClasses="!font-medium px-2 w-7"
				size="xs"
				color="green"
				on:click={acceptDiff}
			>
				<Icon data={faCheck} /></Button
			>
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
				startIcon={{ icon: faClose }}
				{iconOnly}
			>
				Discard
			</Button><Button
				title="Accept generated code"
				btnClasses="!font-medium px-2"
				size="xs"
				color="green"
				on:click={acceptDiff}
				startIcon={{ icon: faCheck }}
				{iconOnly}
			>
				Accept
			</Button>
		</div>
	{/if}
{/if}
{#if ($generatedCode.length === 0 || genLoading) && SUPPORTED_LANGUAGES.has(lang)}
	<Popup
		floatingConfig={{
			middleware: [
				autoPlacement({
					allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
				})
			]
		}}
		let:close
	>
		<svelte:fragment slot="button">
			{#if inlineScript}
				<Button
					size="lg"
					color={genLoading ? 'red' : 'light'}
					btnClasses={genLoading ? '!px-3' : '!px-2 !bg-surface-secondary hover:!bg-surface-hover'}
					nonCaptureEvent={!genLoading}
					on:click={genLoading ? () => abortController?.abort() : undefined}
					bind:element={button}
				>
					{#if genLoading}
						<Icon scale={0.8} data={faCancel} />
					{:else}
						<Icon scale={0.8} data={faMagicWandSparkles} />
					{/if}
				</Button>
			{:else}
				<Button
					title="Generate code from prompt"
					btnClasses="!font-medium"
					size="xs"
					color={genLoading ? 'red' : 'light'}
					spacingSize="md"
					startIcon={genLoading ? undefined : { icon: faMagicWandSparkles }}
					nonCaptureEvent={!genLoading}
					on:click={genLoading ? () => abortController?.abort() : undefined}
					bind:element={button}
				>
					{#if genLoading}
						<WindmillIcon
							white={true}
							class="mr-1 text-white"
							height="16px"
							width="20px"
							spin="veryfast"
						/>
						Cancel
					{:else}
						{isEdit ? 'AI Edit' : 'AI Gen'}
					{/if}
				</Button>
			{/if}
		</svelte:fragment>
		<div class="block text-primary">
			{#if genLoading}
				<div class="w-[42rem] min-h-[3rem] max-h-[34rem] overflow-y-scroll">
					{#if $generatedCode.length > 0}
						<div class="overflow-x-scroll">
							<HighlightCode language={lang} code={$generatedCode} /></div
						>
					{:else}
						<LoadingIcon />
					{/if}
				</div>
			{:else if $existsOpenaiResourcePath}
				<div class="flex w-96">
					<input
						type="text"
						bind:this={input}
						bind:value={funcDesc}
						on:keypress={({ key }) => {
							if (key === 'Enter' && funcDesc.length > 0) {
								onGenerate(() => close(input || null))
							}
						}}
						placeholder={isEdit
							? 'Describe the changes you want'
							: 'Describe what the script should do'}
					/>
					<Button
						size="xs"
						color="blue"
						buttonType="button"
						btnClasses="!p-1 !w-[38px] !ml-2"
						aria-label="Generate"
						on:click={() => {
							onGenerate(() => close(input || null))
						}}
						disabled={funcDesc.length <= 0}
					>
						<Icon data={faMagicWandSparkles} />
					</Button>
				</div>
				{#if ['postgresql', 'mysql'].includes(lang) && dbSchema}
					<div class="flex flex-row items-center justify-between w-96 mt-2">
						<p class="text-sm">
							Will take into account the DB schema
							<Tooltip>
								In order to better generate the script, we pass the selected DB schema to GPT-4.
							</Tooltip>
						</p>
						{#if dbSchema.lang === 'postgresql'}
							<ToggleButtonGroup class="w-auto shrink-0" bind:selected={dbSchema.publicOnly}>
								<ToggleButton value={true} label="Public schema" />
								<ToggleButton value={false} label="All schemas" />
							</ToggleButtonGroup>
						{/if}
					</div>
				{/if}
			{:else}
				<p class="text-sm"
					>Enable Windmill AI in the <a href="/workspace_settings?tab=openai">workspace settings.</a
					></p
				>
			{/if}
		</div>
	</Popup>
{/if}
