<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview } from '$lib/gen'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import type { AppViewerContext, InlineScript } from '../../types'
	import { CornerDownLeft, Maximize2, Trash2 } from 'lucide-svelte'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import { emptySchema, scriptLangToEditorLang } from '$lib/utils'
	import Popover from '../../../Popover.svelte'
	import { computeFields } from './utils'
	import { deepEqual } from 'fast-equals'
	import type { AppInput } from '../../inputType'
	import Kbd from '$lib/components/common/kbd/Kbd.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '../../utils'

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	export let inlineScript: InlineScript
	export let name: string | undefined = undefined
	export let id: string
	export let defaultUserInput: boolean = false
	export let fields: Record<string, AppInput> = {}
	export let syncFields: boolean = false

	const { runnableComponents, stateId, worldStore, state, appPath } =
		getContext<AppViewerContext>('AppViewerContext')

	let editor: Editor
	let simpleEditor: SimpleEditor
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

	$: inlineScript.path = `${appPath}/${name}`

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
		if (inlineScript.schema) {
			loadSchemaAndInputsByName()
		}
	})
	const dispatch = createEventDispatcher()
	let runLoading = false

	async function loadSchemaAndInputsByName() {
		if (syncFields) {
			const newSchema = inlineScript.schema ?? emptySchema()
			const newFields = computeFields(newSchema, defaultUserInput, fields)

			if (!deepEqual(newFields, fields)) {
				fields = newFields
				$stateId++
			}
		}
	}

	let isMac = navigator.userAgent.indexOf('Mac OS X') !== -1

	$: extraLib =
		inlineScript.language == 'frontend' && worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, false, $state, true)
			: undefined

	let refreshOn: string = inlineScript.refreshOn?.map((x) => `${x.id}.${x.key}`).join(' ') ?? ''

	$: handleRefreshOn(refreshOn)

	function handleRefreshOn(refreshOn: string) {
		if (refreshOn && refreshOn != '') {
			inlineScript.refreshOn = refreshOn
				.split(' ')
				.filter((x) => x.split('.').length == 2)
				.map((x) => {
					const [id, key] = x.split('.')
					return { id, key }
				})
		}
	}
</script>

{#if inlineScript.language != 'frontend'}
	<InlineScriptEditorDrawer {editor} bind:this={inlineScriptEditorDrawer} bind:inlineScript />
{/if}

<div class="h-full flex flex-col gap-1">
	<div class="flex justify-between w-full gap-2 px-2 pt-1 flex-row items-center">
		{#if name !== undefined}
			<input
				on:keydown|stopPropagation
				bind:value={name}
				placeholder="Inline script name"
				class="!text-xs !rounded-xs"
			/>
		{/if}
		<div class="flex w-full flex-row gap-2 items-center justify-end">
			{#if validCode}
				<Badge color="green" baseClass="!text-2xs">Valid</Badge>
			{:else}
				<Badge color="red" baseClass="!text-2xs">Invalid</Badge>
			{/if}

			{#if id.startsWith('unused-') || id.startsWith('bg_')}
				<Popover notClickable placement="bottom">
					<Button
						size="xs"
						color="light"
						btnClasses="!px-2 !bg-red-100 hover:!bg-red-200"
						aria-label="Delete"
						on:click={() => dispatch('delete')}
					>
						<Trash2 size={14} class="text-red-800" />
					</Button>
					<svelte:fragment slot="text">Delete</svelte:fragment>
				</Popover>
			{/if}
			{#if inlineScript.language != 'frontend'}
				<Popover notClickable placement="bottom">
					<Button
						size="xs"
						color="light"
						btnClasses="!px-2 !bg-gray-100 hover:!bg-gray-200"
						aria-label="Open full editor"
						on:click={() => {
							inlineScriptEditorDrawer?.openDrawer()
						}}
					>
						<Maximize2 size={14} />
					</Button>
					<svelte:fragment slot="text">Open full editor</svelte:fragment>
				</Popover>
			{/if}

			<Button
				variant="border"
				size="xs"
				color="light"
				btnClasses="!px-2 !py-1"
				on:click={async () => {
					editor?.format()
					simpleEditor?.format()
				}}
			>
				<div class="flex flex-row gap-1 items-center">
					Format

					<div class="flex flex-row items-center gap-1">
						<Kbd>{isMac ? '⌘' : 'CTRL'}</Kbd>
						<Kbd>S</Kbd>
					</div>
				</div>
			</Button>
			{#if $runnableComponents[id] != undefined}
				<Button
					loading={runLoading}
					size="xs"
					color="dark"
					variant="border"
					btnClasses="!px-2 !py-1 !bg-gray-700 !text-white hover:!bg-gray-900"
					on:click={async () => {
						runLoading = true
						await $runnableComponents[id]?.(inlineScript)
						runLoading = false
					}}
				>
					<div class="flex flex-row gap-1 items-center">
						Run

						<div class="flex flex-row items-center gap-1">
							<Kbd>{isMac ? '⌘' : 'CTRL'}</Kbd>
							<Kbd>
								<div class="h-4 flex items-center justify-center">
									<CornerDownLeft size={10} />
								</div>
							</Kbd>
						</div>
					</div>
				</Button>
			{/if}
		</div>
	</div>

	<!-- {inlineScript.content} -->

	<div class="border h-full">
		{#if inlineScript.language != 'frontend'}
			<Editor
				bind:this={editor}
				class="flex flex-1 grow h-full"
				lang={scriptLangToEditorLang(inlineScript?.language)}
				bind:code={inlineScript.content}
				fixedOverflowWidgets={true}
				cmdEnterAction={async () => {
					inlineScript.content = editor?.getCode() ?? ''
					runLoading = true
					await $runnableComponents[id]?.(inlineScript)
					runLoading = false
				}}
				on:change={async (e) => {
					if (inlineScript && inlineScript.language != 'frontend') {
						const oldSchema = JSON.stringify(inlineScript.schema)
						if (inlineScript.schema == undefined) {
							inlineScript.schema = emptySchema()
						}
						await inferInlineScriptSchema(inlineScript?.language, e.detail, inlineScript.schema)
						if (JSON.stringify(inlineScript.schema) != oldSchema) {
							inlineScript = inlineScript
							loadSchemaAndInputsByName()
						}
					}
				}}
			/>
		{:else}
			<SimpleEditor
				bind:this={simpleEditor}
				cmdEnterAction={async () => {
					runLoading = true
					await $runnableComponents[id]?.(inlineScript)
					runLoading = false
				}}
				class="h-full"
				{extraLib}
				bind:code={inlineScript.content}
				lang="javascript"
			/>
		{/if}
	</div>
</div>
