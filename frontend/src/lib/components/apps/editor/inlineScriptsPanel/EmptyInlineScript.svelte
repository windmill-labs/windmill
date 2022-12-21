<script lang="ts">
	import type { Schema } from '$lib/common'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import { Script, type Preview } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fly } from 'svelte/transition'
	import type { AppEditorContext } from '../../types'

	export let name: string

	const { appPath } = getContext<AppEditorContext>('AppEditorContext')
	const dispatch = createEventDispatcher()

	async function inferInlineScriptSchema(
		language: Preview.language,
		content: string,
		schema: Schema
	): Promise<Schema> {
		try {
			await inferArgs(language, content, schema)
		} catch (e) {
			console.error("Couldn't infer args", e)
		}

		return schema
	}

	async function createInlineScriptByLanguage(
		language: Preview.language,
		path: string,
		subkind: 'pgsql' | 'mysql' | undefined = undefined
	) {
		const fullPath = `${appPath}/inline-script/${path}`
		const content = initialCode(language, Script.kind.SCRIPT, subkind ?? 'flow')

		let schema: Schema = emptySchema()

		schema = await inferInlineScriptSchema(language, content, schema)
		const newInlineScript = {
			content,
			language,
			path: fullPath,
			schema
		}
		dispatch('new', newInlineScript)
	}
	const langs = ['deno', 'python3', 'go', 'bash'] as Script.language[]
</script>

<div class="flex flex-col p-4 gap-2 text-sm" in:fly={{ duration: 50 }}>
	Please choose a language:
	<div class="flex gap-2 flex-row flex-wrap">
		{#each langs as lang}
			<FlowScriptPicker
				label={lang}
				{lang}
				on:click={() => {
					createInlineScriptByLanguage(lang, name)
				}}
			/>
		{/each}

		<FlowScriptPicker
			label={`PostgreSQL`}
			lang="pgsql"
			on:click={() => {
				createInlineScriptByLanguage(Script.language.DENO, name, 'pgsql')
			}}
		/>
		<FlowScriptPicker
			label={`MySQL`}
			lang="mysql"
			on:click={() => {
				createInlineScriptByLanguage(Script.language.DENO, name, 'mysql')
			}}
		/>
	</div>
</div>
