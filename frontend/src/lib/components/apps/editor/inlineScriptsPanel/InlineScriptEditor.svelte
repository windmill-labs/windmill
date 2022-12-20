<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Preview, Script } from '$lib/gen'
	import { initialCode } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { faTrash } from '@fortawesome/free-solid-svg-icons'
	import { getContext, onMount } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { CheckCircle, Code2, X } from 'lucide-svelte'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import type { ResultAppInput } from '../../inputType'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { fly } from 'svelte/transition'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer
	export let componentInput: ResultAppInput

	const name =
		componentInput.runnable?.type === 'runnableByName' ? componentInput.runnable?.name : ''

	$: shouldDisplay = componentInput.runnable?.type === 'runnableByName'
	const { appPath } = getContext<AppEditorContext>('AppEditorContext')

	let validCode = false

	async function inferInlineScriptSchema(
		language: Preview.language,
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

	async function createInlineScriptByLanguage(
		language: Preview.language,
		path: string,
		subkind: 'pgsql' | 'mysql' | undefined = undefined
	) {
		const fullPath = `${appPath}/inline-script/${path}`
		const content = initialCode(language, Script.kind.SCRIPT, subkind)

		let schema: Schema = emptySchema()

		schema = await inferInlineScriptSchema(language, content, schema)
		const inlineScript = {
			content,
			language,
			path: fullPath,
			schema
		}
		if (componentInput?.runnable?.type === 'runnableByName') {
			componentInput.runnable.inlineScript = inlineScript
		}
	}

	onMount(async () => {
		if (
			componentInput.runnable?.type === 'runnableByName' &&
			componentInput.runnable.inlineScript
		) {
			componentInput.runnable.inlineScript.schema = await inferInlineScriptSchema(
				componentInput.runnable.inlineScript?.language,
				componentInput.runnable.inlineScript?.content,
				componentInput.runnable.inlineScript?.schema
			)
		}
	})
</script>

{#if componentInput.runnable && componentInput.runnable.type === 'runnableByName' && componentInput.runnable.inlineScript}
	<InlineScriptEditorDrawer
		bind:this={inlineScriptEditorDrawer}
		bind:inlineScript={componentInput.runnable.inlineScript}
	/>
{/if}

{#if shouldDisplay}
	{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.inlineScript}
		<div class="h-full p-4 flex flex-col gap-2" transition:fly={{ duration: 50 }}>
			<div class="flex justify-between w-full flex-row items-center">
				{#if componentInput?.runnable?.name !== undefined}
					<input bind:value={componentInput.runnable.name} placeholder="Inline script name" />
				{/if}
				<div class="flex w-full flex-row gap-2 items-center justify-end">
					{#if validCode}
						<Badge color="green">
							<CheckCircle size={16} />
						</Badge>
					{:else}
						<Badge color="red">
							<X size={16} />
						</Badge>
					{/if}

					<Button
						size="xs"
						color="light"
						variant="border"
						iconOnly
						startIcon={{ icon: faTrash }}
						on:click={() => {
							if (componentInput?.runnable?.type === 'runnableByName') {
								componentInput.runnable = undefined
								componentInput.fields = {}
							}
						}}
					/>
					<Button
						size="xs"
						color="light"
						variant="border"
						on:click={() => {
							inlineScriptEditorDrawer?.openDrawer()
						}}
					>
						<div class="flex gap-1 items-center">
							<Code2 size={16} />
							Open full editor
						</div>
					</Button>
				</div>
			</div>

			{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.inlineScript}
				<div class="border h-full">
					<SimpleEditor
						class="flex flex-1 grow h-full"
						lang="typescript"
						bind:code={componentInput.runnable.inlineScript.content}
						fixedOverflowWidgets={false}
						on:change={async () => {
							if (
								componentInput?.runnable?.type === 'runnableByName' &&
								componentInput?.runnable?.inlineScript
							) {
								let schema = await inferInlineScriptSchema(
									componentInput?.runnable?.inlineScript?.language,
									componentInput.runnable.inlineScript.content,
									componentInput.runnable.inlineScript.schema
								)

								componentInput.runnable.inlineScript.schema = schema

								componentInput = componentInput
							}
						}}
					/>
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col p-4 gap-2 text-sm" in:fly={{ duration: 50 }}>
			Please choose a language:
			<div class="flex gap-2 flex-row flex-wrap">
				{#each Object.values(Script.language) as lang}
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
	{/if}
{/if}
