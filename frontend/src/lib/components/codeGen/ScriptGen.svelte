<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, generateScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { faCheck, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import Popup from '../common/popup/Popup.svelte'
	import { fade } from 'svelte/transition'
	import { Icon } from 'svelte-awesome'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'

	// props
	export let iconOnly: boolean = false
	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined

	// state
	let funcDesc: string = ''
	let genLoading: boolean = false
	let openAIAvailable: boolean | undefined = undefined
	let button: HTMLButtonElement
	let input: HTMLInputElement
	let generatedCode = ''

	async function onGenerate() {
		try {
			// close popup ^^
			const elem = document.activeElement as HTMLElement
			if (elem.blur) {
				elem.blur()
			}

			genLoading = true
			generatedCode = await generateScript({
				language: lang,
				description: funcDesc
			})
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
		generatedCode = ''
	}

	function rejectDiff() {
		generatedCode = ''
	}

	async function checkIfOpenAIAvailable(lang: SupportedLanguage) {
		try {
			const resp = await WorkspaceService.openaiKeyExists({ workspace: $workspaceStore! })
			openAIAvailable = resp.exists && SUPPORTED_LANGUAGES.has(lang)
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

	$: checkIfOpenAIAvailable(lang)

	$: input?.focus()

	$: lang && (generatedCode = '')

	$: generatedCode && showDiff()
	$: !generatedCode && hideDiff()
</script>

{#if openAIAvailable}
	{#if generatedCode}
		<div class="flex gap-1 px-2">
			<Button
				title="Generate code from prompt"
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

			<Button
				title="Generate code from prompt"
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
			</Button></div
		>
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
		>
			Ask AI
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
							if (key === 'Enter') onGenerate()
						}}
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
