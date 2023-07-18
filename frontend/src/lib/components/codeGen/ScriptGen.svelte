<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, editScript, generateScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { faCheck, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import Popup from '../common/popup/Popup.svelte'
	import { fade } from 'svelte/transition'
	import { Icon } from 'svelte-awesome'
	import { existsOpenaiKeyStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Selection } from 'monaco-editor/esm/vs/editor/editor.api'

	// props
	export let iconOnly: boolean = false
	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined

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
			// close popup ^^
			const elem = document.activeElement as HTMLElement
			if (elem.blur) {
				elem.blur()
			}

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

	async function checkIfOpenAIAvailable(lang: SupportedLanguage) {
		try {
			const exists = $existsOpenaiKeyStore
			openAIAvailable = exists && SUPPORTED_LANGUAGES.has(lang)
		} catch (err) {
			console.error(err)
			sendUserToast('Failed to check if OpenAI is available', true)
		}
	}

	function showDiff() {
		diffEditor?.setDiff(editor?.getCode() || '', generatedCode, scriptLangToEditorLang(lang))
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
		<div class="flex gap-1 px-2">
			<Button
				title="Discard generated code"
				btnClasses="!font-medium"
				size="xs"
				color="red"
				spacingSize="xs2"
				on:click={rejectDiff}
				variant="contained"
				startIcon={{ icon: faClose }}
				{iconOnly}
			>
				Discard
			</Button><Button
				title="Accept generated code"
				btnClasses="!font-medium"
				size="xs"
				color="green"
				spacingSize="xs2"
				on:click={acceptDiff}
				startIcon={{ icon: faCheck }}
				{iconOnly}
			>
				Accept
			</Button>
		</div>
	{:else}
		<Button
			title="Generate code from prompt"
			btnClasses="!font-medium text-scondary"
			size="xs"
			color="light"
			spacingSize="md"
			bind:element={button}
			startIcon={{ icon: faMagicWandSparkles }}
			{iconOnly}
			loading={genLoading}
		>
			{isEdit ? 'AI Edit' : 'AI Gen'}
		</Button>
	{/if}
	{#if !generatedCode}
		<Popup
			ref={button}
			options={{ placement: 'top-start' }}
			transition={fade}
			closeOn={[]}
			wrapperClasses="!z-[1002]"
			outerClasses="rounded shadow-xl bg-white border p-3 w-96"
		>
			<label class="block text-gray-900">
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
