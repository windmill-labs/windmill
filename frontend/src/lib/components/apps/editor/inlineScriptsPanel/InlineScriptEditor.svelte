<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview } from '$lib/gen'
	import { faTrash } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, onMount } from 'svelte'
	import type { InlineScript } from '../../types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { CheckCircle, Code2, X } from 'lucide-svelte'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { fly } from 'svelte/transition'
	import Editor from '$lib/components/Editor.svelte'
	import { scriptLangToEditorLang } from '$lib/utils'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	export let inlineScript: InlineScript
	export let name: string | undefined = undefined

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

	onMount(async () => {
		if (inlineScript) {
			inlineScript.schema = await inferInlineScriptSchema(
				inlineScript?.language,
				inlineScript?.content,
				inlineScript?.schema
			)
		}
	})
	const dispatch = createEventDispatcher()
</script>

<InlineScriptEditorDrawer bind:this={inlineScriptEditorDrawer} bind:inlineScript />

<div class="h-full p-4 flex flex-col gap-2" transition:fly={{ duration: 50 }}>
	<div class="flex justify-between w-full gap-1 flex-row items-center">
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
				size="xs"
				color="light"
				variant="border"
				iconOnly
				startIcon={{ icon: faTrash }}
				on:click={() => dispatch('delete')}
			/>
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
			class="flex flex-1 grow h-full"
			lang={scriptLangToEditorLang(inlineScript?.language)}
			bind:code={inlineScript.content}
			fixedOverflowWidgets={false}
			on:change={async () => {
				if (inlineScript) {
					let schema = await inferInlineScriptSchema(
						inlineScript?.language,
						inlineScript.content,
						inlineScript.schema
					)
					inlineScript.schema = schema
					inlineScript = inlineScript
				}
			}}
		/>
	</div>
</div>
