<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview, ScriptLang } from '$lib/gen'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Trash2 } from 'lucide-svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Editor from '$lib/components/Editor.svelte'
	import { emptySchema } from '$lib/utils'

	import { scriptLangToEditorLang } from '$lib/scripts'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import type { AppInput, InlineScript } from '../apps/inputType'
	import CacheTtlPopup from '../apps/editor/inlineScriptsPanel/CacheTtlPopup.svelte'
	import RunButton from '$lib/components/RunButton.svelte'
	import { computeFields } from '../apps/editor/inlineScriptsPanel/utils'
	import EditorBar from '../EditorBar.svelte'
	import { LanguageIcon } from '../common/languageIcons'

	interface Props {
		inlineScript: (InlineScript & { language: ScriptLang }) | undefined
		name?: string | undefined
		id: string
		fields?: Record<string, AppInput>
		path: string
		isLoading?: boolean
		onRun: () => Promise<void>
		onCancel: () => Promise<void>
		editor?: Editor | undefined
		lastDeployedCode?: string | undefined
	}

	let {
		inlineScript = $bindable(),
		name = $bindable(undefined),
		id,
		fields = $bindable(undefined),
		path,
		isLoading = false,
		onRun,
		onCancel,
		editor = $bindable(undefined),
		lastDeployedCode
	}: Props = $props()
	let diffEditor = $state() as DiffEditor | undefined
	let validCode = $state(true)

	async function inferInlineScriptSchema(
		language: Preview['language'],
		content: string,
		schema: Schema
	): Promise<Schema> {
		try {
			await inferArgs(language, content, schema)
			validCode = true
		} catch (e) {
			console.error("Couldn't infer args", e)
			validCode = false
		}

		return schema
	}

	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})

	let diffMode = $state(false)

	function showDiffMode() {
		const model = editor?.getModel()
		if (model == undefined) return
		diffMode = true
		diffEditor?.showWithModelAndOriginal(lastDeployedCode ?? '', model)
		editor?.hide()
	}
	function hideDiffMode() {
		diffMode = false
		diffEditor?.hide()
		editor?.show()
	}

	onMount(async () => {
		if (inlineScript && !inlineScript.schema) {
			inlineScript.schema = await inferInlineScriptSchema(
				inlineScript?.language,
				inlineScript?.content,
				emptySchema()
			)
		}
		syncFields()
	})

	async function syncFields() {
		if (inlineScript) {
			const newSchema = inlineScript.schema ?? emptySchema()
			fields = computeFields(newSchema, true, fields ?? {})
		}
	}

	const dispatch = createEventDispatcher()
	let width = $state(0)
</script>

{#if inlineScript}
	<div class="h-full flex flex-col gap-1" bind:clientWidth={width}>
		<div class="flex justify-between w-full gap-2 px-2 pt-1 flex-row items-center">
			<div class="mx-0.5">
				<LanguageIcon lang={inlineScript.language} width={20} height={20} />
			</div>
			{#if name !== undefined}
				<div class="flex flex-row gap-2 w-full items-center">
					<input
						onkeydown={stopPropagation(bubble('keydown'))}
						bind:value={name}
						placeholder="Inline script name"
						class="!text-xs !rounded-sm !shadow-none"
					/>
				</div>
				<Button
					title="Clear script"
					size="xs2"
					color="light"
					variant="contained"
					aria-label="Clear script"
					on:click={() => dispatch('delete')}
					endIcon={{ icon: Trash2 }}
					iconOnly
				/>
			{/if}
			<div class="flex w-full flex-row gap-2 items-center justify-end">
				{#if inlineScript}
					<CacheTtlPopup bind:cache_ttl={inlineScript.cache_ttl} />
				{/if}

				<Button
					variant="default"
					size="xs2"
					on:click={async () => {
						editor?.format()
					}}
				>
					Format
				</Button>
				<RunButton {isLoading} {onRun} {onCancel} />
			</div>
		</div>

		<div class="shadow-sm px-1 border-b-1 border-gray-200 dark:border-gray-700">
			<EditorBar
				{validCode}
				{editor}
				lang={inlineScript.language}
				{websocketAlive}
				iconOnly={width < 950}
				kind={'script'}
				template={'script'}
				on:showDiffMode={showDiffMode}
				on:hideDiffMode={hideDiffMode}
				{lastDeployedCode}
				{diffMode}
				openAiChat
				moduleId={id}
			/>
		</div>

		<div class="border-y h-full w-full relative">
			<Editor
				path={path + '/' + id}
				bind:this={editor}
				class="flex flex-1 grow h-full"
				scriptLang={inlineScript.language}
				bind:code={inlineScript.content}
				fixedOverflowWidgets={true}
				cmdEnterAction={() => onRun()}
				bind:websocketAlive
				rawAppRunnableKey={id}
				on:change={async (e) => {
					if (inlineScript) {
						if (inlineScript.lock != undefined) {
							inlineScript.lock = undefined
						}
						const oldSchema = JSON.stringify(inlineScript.schema)
						if (inlineScript.schema == undefined) {
							inlineScript.schema = emptySchema()
						}
						await inferInlineScriptSchema(inlineScript?.language, e.detail, inlineScript.schema)
						if (JSON.stringify(inlineScript.schema) != oldSchema) {
							inlineScript = inlineScript
							syncFields()
						}
					}
					// $app = $app
				}}
				args={Object.entries(fields ?? {}).reduce((acc, [key, obj]) => {
					acc[key] = obj.type === 'static' ? obj.value : undefined
					return acc
				}, {})}
			/>

			<DiffEditor
				open={false}
				bind:this={diffEditor}
				modifiedModel={editor?.getModel()}
				className="h-full"
				automaticLayout
				fixedOverflowWidgets
				defaultLang={scriptLangToEditorLang(inlineScript?.language)}
			/>
		</div>
	</div>
{/if}
