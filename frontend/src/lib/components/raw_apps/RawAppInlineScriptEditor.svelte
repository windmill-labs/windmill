<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview } from '$lib/gen'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Maximize2, Trash2 } from 'lucide-svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Editor from '$lib/components/Editor.svelte'
	import { emptySchema } from '$lib/utils'

	import { scriptLangToEditorLang } from '$lib/scripts'
	import ScriptGen from '$lib/components/copilot/ScriptGen.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import EditorSettings from '$lib/components/EditorSettings.svelte'
	import InlineScriptEditorDrawer from '../apps/editor/inlineScriptsPanel/InlineScriptEditorDrawer.svelte'
	import type { InlineScript } from '../apps/types'
	import type { AppInput } from '../apps/inputType'
	import CacheTtlPopup from '../apps/editor/inlineScriptsPanel/CacheTtlPopup.svelte'
	import RunButton from '$lib/components/RunButton.svelte'
	import { computeFields } from '../apps/editor/inlineScriptsPanel/utils'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	export let inlineScript: InlineScript | undefined
	export let name: string | undefined = undefined
	export let id: string
	export let fields: Record<string, AppInput> = {}
	export let path: string
	export let isLoading: boolean = false
	export let onRun: () => Promise<void>
	export let onCancel: () => Promise<void>

	export let editor: Editor | undefined = undefined
	let diffEditor: DiffEditor
	let validCode = true

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

	onMount(async () => {
		if (inlineScript && !inlineScript.schema) {
			if (inlineScript.language != 'frontend') {
				inlineScript.schema = await inferInlineScriptSchema(
					inlineScript?.language,
					inlineScript?.content,
					emptySchema()
				)
			}
		}
		syncFields()
	})

	async function syncFields() {
		if (inlineScript) {
			const newSchema = inlineScript.schema ?? emptySchema()
			fields = computeFields(newSchema, true, fields)
		}
	}

	const dispatch = createEventDispatcher()

	let drawerIsOpen: boolean | undefined = undefined
</script>

{#if inlineScript}
	{#if inlineScript.language != 'frontend'}
		<InlineScriptEditorDrawer
			{id}
			appPath={path}
			bind:isOpen={drawerIsOpen}
			{editor}
			bind:this={inlineScriptEditorDrawer}
			bind:inlineScript
			on:createScriptFromInlineScript={() => {
				dispatch('createScriptFromInlineScript')
				drawerIsOpen = false
			}}
		/>
	{/if}
	<div class="h-full flex flex-col gap-1">
		<div class="flex justify-between w-full gap-2 px-2 pt-1 flex-row items-center">
			{#if name !== undefined}
				<div class="flex flex-row gap-2 w-full items-center">
					<input
						on:keydown|stopPropagation
						bind:value={name}
						placeholder="Inline script name"
						class="!text-xs !rounded-sm !shadow-none"
						on:keyup={() => {
							// $app = $app
							// if (stateId) {
							// 	$stateId++
							// }
						}}
					/>
					<div
						title={validCode ? 'Main function parsable' : 'Main function not parsable'}
						class="rounded-full !w-2 !h-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
					></div>
				</div>
			{/if}
			<div class="flex w-full flex-row gap-1 items-center justify-end">
				{#if inlineScript}
					<CacheTtlPopup bind:cache_ttl={inlineScript.cache_ttl} />
				{/if}
				<ScriptGen
					lang={inlineScript?.language}
					{editor}
					{diffEditor}
					inlineScript
					args={Object.entries(fields ?? {}).reduce((acc, [key, obj]) => {
						acc[key] = obj.type === 'static' ? obj.value : undefined
						return acc
					}, {})}
				/>
				<EditorSettings />

				<Button
					title="Delete"
					size="xs2"
					color="light"
					variant="contained"
					aria-label="Delete"
					on:click={() => dispatch('delete')}
					endIcon={{ icon: Trash2 }}
					iconOnly
				/>
				{#if inlineScript.language != 'frontend'}
					<Button
						size="xs2"
						color="light"
						title="Full Editor"
						variant="contained"
						on:click={() => {
							inlineScriptEditorDrawer?.openDrawer()
						}}
						endIcon={{ icon: Maximize2 }}
						iconOnly
					/>
				{/if}

				<Button
					variant="border"
					size="xs2"
					color="light"
					on:click={async () => {
						editor?.format()
					}}
					shortCut={{
						key: 'S'
					}}
				>
					Format
				</Button>
				<RunButton {isLoading} {onRun} {onCancel} />
			</div>
		</div>

		<!-- {inlineScript.content} -->

		<div class="border-y h-full w-full">
			{#if !drawerIsOpen && inlineScript.language != 'frontend'}
				<Editor
					path={path + '/' + id}
					bind:this={editor}
					small
					class="flex flex-1 grow h-full"
					scriptLang={inlineScript.language}
					bind:code={inlineScript.content}
					fixedOverflowWidgets={true}
					cmdEnterAction={() => onRun()}
					on:change={async (e) => {
						if (inlineScript && inlineScript.language != 'frontend') {
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
					class="h-full"
					automaticLayout
					fixedOverflowWidgets
					defaultLang={scriptLangToEditorLang(inlineScript?.language)}
				/>
			{/if}
		</div>
	</div>
{/if}
