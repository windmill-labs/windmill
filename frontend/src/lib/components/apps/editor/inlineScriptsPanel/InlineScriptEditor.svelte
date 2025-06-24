<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Preview } from '$lib/gen'
	import { createEventDispatcher, getContext, onMount, untrack } from 'svelte'
	import type { AppViewerContext, InlineScript } from '../../types'
	import { Maximize2, Trash2 } from 'lucide-svelte'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import { inferArgs, parseOutputs } from '$lib/infer'
	import type { Schema } from '$lib/common'
	import Editor from '$lib/components/Editor.svelte'
	import { defaultIfEmptyString, editorPositionMap, emptySchema, itemsExists } from '$lib/utils'
	import { computeFields } from './utils'
	import { deepEqual } from 'fast-equals'
	import type { AppInput } from '../../inputType'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { buildExtraLib } from '../../utils'
	import RunButton from './AppRunButton.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import ScriptGen from '$lib/components/copilot/ScriptGen.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import CacheTtlPopup from './CacheTtlPopup.svelte'
	import EditorSettings from '$lib/components/EditorSettings.svelte'
	import { userStore } from '$lib/stores'

	const {
		runnableComponents,
		stateId,
		worldStore,
		state: stateStore,
		appPath,
		app
	} = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		inlineScript: InlineScript | undefined
		name?: string | undefined
		id: string
		defaultUserInput?: boolean
		fields?: Record<string, AppInput>
		syncFields?: boolean
		transformer?: boolean
		componentType?: string | undefined
		editor?: Editor | undefined
	}

	let {
		inlineScript = $bindable(),
		name = $bindable(undefined),
		id,
		defaultUserInput = false,
		fields = $bindable({}),
		syncFields = false,
		transformer = false,
		componentType = undefined,
		editor = $bindable(undefined)
	}: Props = $props()
	let diffEditor: DiffEditor | undefined = $state()
	let simpleEditor: SimpleEditor | undefined = $state()
	let validCode = $state(true)
	let inlineScriptEditorDrawer: InlineScriptEditorDrawer | undefined = $state()

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

	function onNameChange() {
		if (inlineScript) {
			inlineScript.path = `${defaultIfEmptyString(
				$appPath,
				`u/${$userStore?.username ?? 'unknown'}/newapp`
			)}/${name?.replaceAll(' ', '_')}`
		}
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
		if (inlineScript?.schema && inlineScript.language != 'frontend') {
			loadSchemaAndInputsByName()
		}
		if (inlineScript?.language == 'frontend' && inlineScript.content) {
			inferSuggestions(inlineScript.content)
		}
		if (!inlineScript?.path) {
			onNameChange()
		}
	})

	const dispatch = createEventDispatcher()
	let runLoading = $state(false)

	function preConnect(newFields) {
		if (!componentType) {
			return
		}

		if (componentType === 'steppercomponent') {
			const componentOutputs = $worldStore?.outputsById[id]

			if (componentOutputs.currentStepIndex) {
				newFields['stepIndex'] = {
					type: 'evalv2',
					expr: `${id}.currentStepIndex`,
					fieldType: 'number'
				}
			}
		} else if (
			componentType === 'aggridinfinitecomponent' ||
			componentType === 'aggridinfinitecomponentee'
		) {
			newFields['offset'] = {
				type: 'evalv2',
				expr: `${id}.params.offset`,
				fieldType: 'number'
			}
			newFields['limit'] = {
				type: 'evalv2',
				expr: `${id}.params.limit`,
				fieldType: 'number'
			}
			newFields['orderBy'] = {
				type: 'evalv2',
				expr: `${id}.params.orderBy`,
				fieldType: 'string'
			}
			newFields['isDesc'] = {
				type: 'evalv2',
				expr: `${id}.params.isDesc`,
				fieldType: 'boolean'
			}
			newFields['search'] = {
				type: 'evalv2',
				expr: `${id}.params.search`,
				fieldType: 'string'
			}
		}
	}

	function assertConnections(newFields) {
		if (
			componentType === 'aggridinfinitecomponent' ||
			componentType === 'aggridinfinitecomponentee'
		) {
			const fields = ['offset', 'limit', 'orderBy', 'isDesc', 'search']

			fields.forEach((field) => {
				if (newFields[field]?.type !== 'evalv2') {
					newFields[field] = {
						type: 'evalv2',
						expr: `${id}.params.${field}`,
						fieldType: newFields[field]?.fieldType ?? 'string'
					}
				}
			})
		}
	}

	async function loadSchemaAndInputsByName() {
		if (syncFields && inlineScript) {
			const newSchema = inlineScript.schema ?? emptySchema()
			const hadPreviousFields = Object.keys(fields).length > 0
			const newFields = computeFields(newSchema, defaultUserInput, fields)

			// First time we load the schema, we want to trigger the pre-connect
			if (!hadPreviousFields && Object.keys(newFields).length > 0) {
				preConnect(newFields)
			}

			assertConnections(newFields)

			if (!deepEqual(newFields, fields)) {
				fields = newFields
				if (stateId) {
					$stateId++
				}
			}
			$app = $app
		}
	}

	// 	`
	// /** The current's app state */
	// const state: Record<string, any>;`

	let drawerIsOpen: boolean | undefined = $state(undefined)

	async function inferSuggestions(code: string) {
		const outputs = await parseOutputs(code, true)
		if (outputs && inlineScript) {
			inlineScript.suggestedRefreshOn = []
			for (const [id, key] of outputs) {
				if (
					key in ($worldStore?.outputsById[id] ?? {}) &&
					!itemsExists(inlineScript.refreshOn, { key, id }) &&
					!itemsExists(inlineScript.suggestedRefreshOn, { key, id })
				) {
					inlineScript.suggestedRefreshOn = [
						...(inlineScript?.suggestedRefreshOn ?? []),
						{ key, id }
					]
				}
			}
			if (stateId) {
				$stateId++
			}
		}
	}

	let lastName = $state(name)
	$effect(() => {
		if (name && name !== lastName) {
			lastName = name
			untrack(() => onNameChange())
		}
	})

	let isFrontend = $derived(inlineScript?.language == 'frontend')
	let extraLib = $derived(
		isFrontend && worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, id, $stateStore, true)
			: undefined
	)
</script>

<!-- <pre class="text-2xs">{JSON.stringify($worldStore?.outputsById, null, 2)}</pre> -->
{#if inlineScript}
	{#if inlineScript.language != 'frontend'}
		<InlineScriptEditorDrawer
			{id}
			appPath={$appPath}
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
				{#if !transformer}
					<div class="flex flex-row gap-2 w-full items-center">
						<input
							onkeydown={stopPropagation(bubble('keydown'))}
							bind:value={name}
							placeholder="Inline script name"
							class="!text-xs !rounded-sm !shadow-none"
							onkeyup={() => {
								$app = $app
								if (stateId) {
									$stateId++
								}
							}}
						/>
						<div
							title={validCode ? 'Main function parsable' : 'Main function not parsable'}
							class="rounded-full !w-2 !h-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
						></div>
					</div>
				{:else}
					<span class="text-xs font-semibold truncate w-full">{name} of {id}</span>
				{/if}
			{/if}
			<div class="flex w-full flex-row gap-1 items-center justify-end">
				{#if inlineScript}
					<CacheTtlPopup bind:cache_ttl={inlineScript.cache_ttl} />
				{/if}
				<ScriptGen
					lang={inlineScript?.language}
					editor={inlineScript?.language === 'frontend' ? simpleEditor : editor}
					{diffEditor}
					inlineScript
					args={Object.entries(fields).reduce((acc, [key, obj]) => {
						acc[key] = obj.type === 'static' ? obj.value : undefined
						return acc
					}, {})}
					{transformer}
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
						simpleEditor?.format()
					}}
					shortCut={{
						key: 'S'
					}}
				>
					Format
				</Button>
				<RunButton bind:runLoading {id} inlineScript={!transformer ? inlineScript : undefined} />
			</div>
		</div>

		<!-- {inlineScript.content} -->

		<div class="border-y h-full w-full">
			{#if !drawerIsOpen}
				{#if inlineScript.language != 'frontend'}
					<Editor
						path={$appPath + '/' + id}
						bind:this={editor}
						small
						class="flex flex-1 grow h-full"
						scriptLang={inlineScript.language}
						bind:code={inlineScript.content}
						fixedOverflowWidgets={true}
						cmdEnterAction={async () => {
							if (inlineScript) {
								inlineScript.content = editor?.getCode() ?? ''
							}
							try {
								runLoading = true
								await Promise.all(
									$runnableComponents[id]?.cb?.map((f) => f?.(inlineScript, true)) ?? []
								)
							} catch {}
							runLoading = false
						}}
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
									loadSchemaAndInputsByName()
								}
							}
							$app = $app
						}}
						initialCursorPos={editorPositionMap[`inline-${id}`]}
						on:cursorPositionChange={(e) => (editorPositionMap[`inline-${id}`] = e.detail.position)}
						args={Object.entries(fields).reduce((acc, [key, obj]) => {
							acc[key] = obj.type === 'static' ? obj.value : undefined
							return acc
						}, {})}
					/>
				{:else}
					<SimpleEditor
						bind:this={simpleEditor}
						class="h-full max-w-full"
						small
						allowVim
						{extraLib}
						bind:code={inlineScript.content}
						lang="javascript"
						domLib
						cmdEnterAction={async () => {
							runLoading = true
							await await Promise.all(
								$runnableComponents[id]?.cb?.map((f) =>
									f(!transformer ? inlineScript : undefined, true)
								)
							)
							runLoading = false
						}}
						on:change={async (e) => {
							inferSuggestions(e.detail.code)
							$app = $app
						}}
						initialCursorPos={editorPositionMap[`inline-${id}`]}
						on:cursorPositionChange={(e) => (editorPositionMap[`inline-${id}`] = e.detail.position)}
					/>
				{/if}

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
