<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, fixScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { faCheck, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'

	// props
	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined
	export let error: string

	// state
	let genLoading: boolean = false
	let openAIAvailable: boolean | undefined = undefined
	let generatedCode = ''

	async function onFix() {
		if (!error) {
			return
		}
		try {
			// close popup ^^
			const elem = document.activeElement as HTMLElement
			if (elem.blur) {
				elem.blur()
			}

			genLoading = true
			generatedCode = await fixScript({
				language: lang,
				code: editor?.getCode() || '',
				error
			})
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
		error = ''
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

	$: lang && (generatedCode = '')

	$: generatedCode && showDiff()
	$: !generatedCode && hideDiff()
</script>

{#if error}
	{#if openAIAvailable}
		<div class="mt-2">
			{#if generatedCode}
				<div class="flex gap-1">
					<Button
						title="Discard generated code"
						size="xs"
						color="red"
						spacingSize="xs2"
						on:click={rejectDiff}
						variant="contained"
						startIcon={{ icon: faClose }}
					>
						Discard
					</Button><Button
						title="Accept generated code"
						size="xs"
						color="green"
						spacingSize="xs2"
						on:click={acceptDiff}
						startIcon={{ icon: faCheck }}
					>
						Accept
					</Button>
				</div>
			{:else}
				<Button
					title="Generate code from prompt"
					size="xs"
					color="blue"
					spacingSize="xs2"
					startIcon={{ icon: faMagicWandSparkles }}
					loading={genLoading}
					on:click={onFix}
				>
					AI Fix
				</Button>
			{/if}</div
		>
	{/if}
{/if}
