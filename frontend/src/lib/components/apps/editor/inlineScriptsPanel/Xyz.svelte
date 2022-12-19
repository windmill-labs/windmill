<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Preview, Script } from '$lib/gen'
	import { initialCode } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { faTrash } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { Code2 } from 'lucide-svelte'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import type { ResultAppInput } from '../../inputType'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer
	export let componentInput: ResultAppInput
	export let selectedScriptName: string | undefined = undefined

	$: shouldDisplay =
		componentInput.runnable?.type === 'runnableByName' &&
		componentInput.runnable?.name === selectedScriptName
	const { appPath } = getContext<AppEditorContext>('AppEditorContext')

	function createInlineScriptByLanguage(
		language: Preview.language,
		path: string,
		subkind: 'pgsql' | 'mysql' | undefined = undefined
	): void {
		const fullPath = `${appPath}/inline-script/${path}`

		const inlineScript = {
			content: initialCode(language, Script.kind.SCRIPT, subkind),
			language: language,
			path: fullPath,
			schema: emptySchema()
		}
		if (componentInput?.runnable?.type === 'runnableByName') {
			componentInput.runnable.inlineScript = inlineScript
		}
	}
</script>

{#if componentInput.runnable && componentInput.runnable.type === 'runnableByName' && componentInput.runnable.inlineScript}
	<InlineScriptEditorDrawer
		bind:this={inlineScriptEditorDrawer}
		bind:inlineScript={componentInput.runnable.inlineScript}
	/>
{/if}

{#if shouldDisplay}
	{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.inlineScript}
		<div class="h-full p-4 flex flex-col gap-2 ">
			<div class="flex w-full flex-row-reverse gap-2 items-center">
				<Button
					size="xs"
					color="light"
					variant="border"
					on:click={() => {
						if (selectedScriptName) {
							inlineScriptEditorDrawer?.openDrawer()
						}
					}}
				>
					<div class="flex gap-1 items-center">
						<Code2 size={16} />
						Open full editor
					</div>
				</Button>
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
			</div>

			{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.inlineScript}
				<div class="border h-full">
					<SimpleEditor
						class="flex flex-1 grow h-full"
						lang="typescript"
						bind:code={componentInput.runnable.inlineScript.content}
						fixedOverflowWidgets={false}
					/>
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col p-4 gap-2 text-sm">
			Please choose a language:
			<div class="flex gap-2 flex-row flex-wrap">
				{#each Object.values(Script.language) as lang}
					<FlowScriptPicker
						label={lang}
						{lang}
						on:click={() => {
							if (selectedScriptName) {
								createInlineScriptByLanguage(lang, selectedScriptName)
							}
						}}
					/>
				{/each}

				<FlowScriptPicker
					label={`PostgreSQL`}
					lang="pgsql"
					on:click={() => {
						if (selectedScriptName) {
							createInlineScriptByLanguage(Script.language.DENO, selectedScriptName, 'pgsql')
						}
					}}
				/>
				<FlowScriptPicker
					label={`MySQL`}
					lang="mysql"
					on:click={() => {
						if (selectedScriptName) {
							createInlineScriptByLanguage(Script.language.DENO, selectedScriptName, 'mysql')
						}
					}}
				/>
			</div>
		</div>
	{/if}
{/if}
