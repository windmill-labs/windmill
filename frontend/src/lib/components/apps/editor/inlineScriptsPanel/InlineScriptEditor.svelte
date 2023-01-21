<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview } from '$lib/gen'
	import { faTrash } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import type { AppEditorContext, InlineScript } from '../../types'
	import { CheckCircle, Code2, X } from 'lucide-svelte'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { fly } from 'svelte/transition'
	import Editor from '$lib/components/Editor.svelte'
	import { emptySchema, scriptLangToEditorLang } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	export let inlineScript: InlineScript
	export let name: string | undefined = undefined
	export let id: string

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	let editor: Editor
	let validCode = true

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

	onMount(async () => {
		if (inlineScript && !inlineScript.schema) {
			inlineScript.schema = await inferInlineScriptSchema(
				inlineScript?.language,
				inlineScript?.content,
				emptySchema()
			)
		}
	})
	const dispatch = createEventDispatcher()
	let runLoading = false
</script>

<InlineScriptEditorDrawer {editor} bind:this={inlineScriptEditorDrawer} bind:inlineScript />

<div class="h-full flex flex-col gap-1" transition:fly|local={{ duration: 50 }}>
	<div class="flex justify-between w-full gap-1 px-2 pt-1 flex-row items-center">
		{#if name !== undefined}
			<input bind:value={name} placeholder="Inline script name" />
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
				variant="border"
				size="xs"
				color="blue"
				on:click={async () => {
					editor.format()
				}}
			>
				Format&nbsp;<Tooltip>Ctrl+S</Tooltip>
			</Button>
			{#if id.startsWith('unused-') || id.startsWith('bg_')}
				<Button
					size="xs"
					color="light"
					variant="border"
					iconOnly
					startIcon={{ icon: faTrash }}
					on:click={() => dispatch('delete')}
				/>
			{/if}
			{#if $runnableComponents[id] != undefined}
				<Button
					loading={runLoading}
					size="xs"
					color="blue"
					on:click={async () => {
						runLoading = true
						await $runnableComponents[id]?.()
						runLoading = false
					}}
				>
					Run&nbsp;
					<Tooltip light>Ctrl+Enter</Tooltip>
				</Button>
			{/if}

			<Button
				size="xs"
				color="blue"
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

	<div class="border h-full">
		<Editor
			bind:this={editor}
			class="flex flex-1 grow h-full"
			lang={scriptLangToEditorLang(inlineScript?.language)}
			bind:code={inlineScript.content}
			fixedOverflowWidgets={true}
			cmdEnterAction={async () => {
				runLoading = true
				await $runnableComponents[id]?.()
				runLoading = false
			}}
			on:change={async (e) => {
				if (inlineScript) {
					const oldSchema = JSON.stringify(inlineScript.schema)
					if (inlineScript.schema == undefined) {
						inlineScript.schema = emptySchema()
					}
					await inferInlineScriptSchema(inlineScript?.language, e.detail, inlineScript.schema)
					if (JSON.stringify(inlineScript.schema) != oldSchema) {
						inlineScript = inlineScript
					}
				}
			}}
		/>
	</div>
</div>
