<script lang="ts">
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES, editScript, generateScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'
	import { faCheck, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import Popup from '../common/popup/Popup.svelte'
	import { Icon } from 'svelte-awesome'
	import { dbSchema, existsOpenaiResourcePath } from '$lib/stores'
	import type DiffEditor from '../DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Selection } from 'monaco-editor/esm/vs/editor/editor.api'
	import type SimpleEditor from '../SimpleEditor.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'

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
	let generatedCode = ''
	let selection: Selection | undefined
	let isEdit = false

	async function onGenerate() {
		if (funcDesc.length <= 0) {
			return
		}
		try {
			genLoading = true
			if (isEdit && selection) {
				const selectedCode = editor?.getSelectedLines() || ''
				const originalCode = editor?.getCode() || ''
				const result = await editScript({
					language: lang,
					description: funcDesc,
					selectedCode,
					dbSchema: $dbSchema
				})
				generatedCode = originalCode.replace(selectedCode, result.code + '\n')
			} else {
				const result = await generateScript({
					language: lang,
					description: funcDesc,
					dbSchema: $dbSchema
				})
				generatedCode = result.code
			}
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
		generatedCode = ''
	}

	function rejectDiff() {
		generatedCode = ''
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

	$: input?.focus()

	$: lang && (generatedCode = '')

	$: generatedCode && showDiff()
	$: !generatedCode && hideDiff()
	$: editor && setSelectionHandler()
	$: selection && (isEdit = !selection.isEmpty())
</script>

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
{#if !generatedCode && SUPPORTED_LANGUAGES.has(lang)}
	<Popup floatingConfig={{ placement: 'bottom-end', strategy: 'absolute' }} let:close>
		<svelte:fragment slot="button">
			{#if inlineScript}
				<Button
					size="lg"
					color="light"
					btnClasses="!px-2 !bg-surface-secondary hover:!bg-surface-hover"
					loading={genLoading}
					nonCaptureEvent={true}
				>
					<Icon scale={0.8} data={faMagicWandSparkles} />
				</Button>
			{:else}
				<Button
					title="Generate code from prompt"
					btnClasses="!font-medium text-secondary"
					size="xs"
					color="light"
					spacingSize="md"
					startIcon={{ icon: faMagicWandSparkles }}
					{iconOnly}
					loading={genLoading}
					nonCaptureEvent={true}
				>
					{isEdit ? 'AI Edit' : 'AI Gen'}
				</Button>
			{/if}
		</svelte:fragment>
		<label class="block text-primary">
			{#if $existsOpenaiResourcePath}
				<div class="flex w-96">
					<input
						type="text"
						bind:this={input}
						bind:value={funcDesc}
						on:keypress={({ key }) => {
							if (key === 'Enter' && funcDesc.length > 0) {
								close(input || null)
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
						btnClasses="!p-1 !w-[38px] !ml-2"
						aria-label="Generate"
						on:click={() => {
							close(input || null)
							onGenerate()
						}}
						disabled={funcDesc.length <= 0}
					>
						<Icon data={faMagicWandSparkles} />
					</Button>
				</div>
				{#if ['postgresql', 'mysql'].includes(lang) && $dbSchema}
					<div class="flex flex-row items-center justify-between w-96 mt-2">
						<p class="text-sm">
							Will take into account the DB schema
							<Tooltip>
								In order to better generate the script, we pass the selected DB schema to GPT-4.
							</Tooltip>
						</p>
						{#if $dbSchema.lang === 'postgresql'}
							<ToggleButtonGroup class="w-auto shrink-0" bind:selected={$dbSchema.publicOnly}>
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
		</label>
	</Popup>
{/if}
