<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, editScript, generateScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { faCheck, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import Popup from '../common/popup/Popup.svelte'
	import { Icon } from 'svelte-awesome'
	import { existsOpenaiKeyStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Selection } from 'monaco-editor/esm/vs/editor/editor.api'
	import type SimpleEditor from '../SimpleEditor.svelte'

	// props
	export let iconOnly: boolean = false
	export let lang: SupportedLanguage | 'frontend'
	export let editor: Editor | SimpleEditor | undefined
	export let diffEditor: DiffEditor | undefined
	export let inlineScript = false

	// state
	let funcDesc: string = ''
	let genLoading: boolean = false
	let openAIAvailable: boolean | undefined = undefined
	let button: HTMLButtonElement | undefined
	let input: HTMLInputElement | undefined
	let generatedCode = ''
	let selection: Selection | undefined
	let isEdit = false

	async function onGenerate() {
		try {
			genLoading = true
			if (isEdit && selection) {
				const selectedCode = editor?.getSelectedLines() || ''
				const originalCode = editor?.getCode() || ''
				const selectionGenCode = await editScript({
					language: lang,
					description: funcDesc,
					selectedCode
				})
				generatedCode = originalCode.replace(selectedCode, selectionGenCode + '\n')
			} else {
				generatedCode = await generateScript({
					language: lang,
					description: funcDesc
				})
			}
			funcDesc = ''
		} catch (err) {
			sendUserToast('Failed to generate code', true)
			console.error(err)
		} finally {
			genLoading = false
		}
	}

	function acceptDiff() {
		editor?.setCode(diffEditor?.getModified() || '')
		editor?.format()
		generatedCode = ''
	}

	function rejectDiff() {
		generatedCode = ''
	}

	async function checkIfOpenAIAvailable(lang: SupportedLanguage | 'frontend') {
		try {
			const exists = $existsOpenaiKeyStore
			openAIAvailable = exists && SUPPORTED_LANGUAGES.has(lang)
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to check if OpenAI is available', true)
		}
	}

	function showDiff() {
		diffEditor?.setDiff(
			editor?.getCode() || '',
			generatedCode,
			lang === 'frontend' ? 'javascript' : scriptLangToEditorLang(lang)
		)
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

	$: checkIfOpenAIAvailable(lang)

	$: input?.focus()

	$: lang && (generatedCode = '')

	$: generatedCode && showDiff()
	$: !generatedCode && hideDiff()
	$: editor && setSelectionHandler()
	$: selection && (isEdit = !selection.isEmpty())
</script>

{#if openAIAvailable}
	{#if generatedCode}
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
	{#if !generatedCode && !genLoading}
		<Popup floatingConfig={{ placement: 'bottom-end', strategy: 'absolute' }}>
			<svelte:fragment slot="button">
				{#if inlineScript}
					<Button
						size="lg"
						bind:element={button}
						color="light"
						btnClasses="!px-2 !bg-gray-100 hover:!bg-gray-200"
						loading={genLoading}
						nonCaptureEvent={true}
					>
						<Icon scale={0.8} data={faMagicWandSparkles} />
					</Button>
				{:else}
					<Button
						title="Generate code from prompt"
						btnClasses="!font-medium text-gray-600"
						size="xs"
						color="light"
						spacingSize="md"
						bind:element={button}
						startIcon={{ icon: faMagicWandSparkles }}
						{iconOnly}
						loading={genLoading}
						nonCaptureEvent={true}
					>
						{isEdit ? 'AI Edit' : 'AI Gen'}
					</Button>
				{/if}
			</svelte:fragment>
			<label class="block text-gray-900 w-96">
				<div class="pb-1 text-sm text-gray-600">Prompt</div>
				<div class="flex w-full">
					<input
						type="text"
						bind:this={input}
						bind:value={funcDesc}
						class="!w-auto grow"
						on:keypress={({ key }) => {
							if (key === 'Enter') {
								onGenerate()
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
						btnClasses="!p-1 !w-[34px] !ml-1"
						aria-label="Generate"
						on:click={onGenerate}
					>
						<Icon data={faMagicWandSparkles} />
					</Button>
				</div>
			</label>
		</Popup>
	{/if}
{/if}
