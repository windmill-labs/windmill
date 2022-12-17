<script lang="ts">
	import type { Schema } from '$lib/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Preview, Script } from '$lib/gen'
	import { initialCode } from '$lib/script_helpers'
	import { classNames, emptySchema } from '$lib/utils'
	import { faTrash } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import { Code2 } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import InlineScriptEditorDrawer from './InlineScriptEditorDrawer.svelte'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'

	const { app, appPath, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	$: selectedScriptName = undefined as string | undefined
	$: selectedScript = undefined as
		| { content: string; language: Preview.language; path: string; schema: Schema }
		| undefined

	selectedComponent.subscribe((selectedComponentId: string | undefined) => {
		if (selectedComponentId) {
			const selectedComponent = $app.grid.find(
				(gridComponent) => gridComponent?.data?.id === selectedComponentId
			)
			if (
				selectedComponent?.data?.componentInput?.type === 'runnable' &&
				selectedComponent.data.componentInput.runnable !== undefined &&
				selectedComponent.data.componentInput.runnable.type === 'runnableByName'
			) {
				const inlineScriptName = selectedComponent.data.componentInput.runnable.inlineScriptName
				selectedScript = $app.inlineScripts[inlineScriptName]
				selectedScriptName = inlineScriptName
			}
		}
	})

	$: scriptsUsedByComponents = new Map<string, string>()
	$: {
		scriptsUsedByComponents.clear()

		$app.grid
			.filter((gridComponent) => gridComponent)
			.forEach((gridComponent) => {
				if (
					gridComponent.data?.componentInput?.type === 'runnable' &&
					gridComponent.data.componentInput.runnable !== undefined &&
					gridComponent.data.componentInput.runnable.type === 'runnableByName'
				) {
					scriptsUsedByComponents.set(
						gridComponent.data.componentInput.runnable.inlineScriptName,
						gridComponent.data.id
					)
				}
			})
	}

	function deleteInlineScript() {
		const key = Object.keys($app.inlineScripts).find(
			(key) => $app.inlineScripts[key]?.path === selectedScript?.path
		)

		if (key && $app.inlineScripts[key]) {
			delete $app.inlineScripts[key]
			$app = $app
			selectedScript = undefined
		}
	}

	function createInlineScriptByLanguage(
		language: Preview.language,
		path: string,
		subkind: 'pgsql' | 'mysql' | undefined = undefined
	) {
		const fullPath = `${appPath}/inline-script/${path}`

		const inlineScript = {
			content: initialCode(language, Script.kind.SCRIPT, subkind),
			language: language,
			path: fullPath,
			schema: emptySchema()
		}

		selectedScript = inlineScript

		$app.inlineScripts = {
			...$app.inlineScripts,
			[path]: inlineScript
		}
	}

	let inlineScriptEditorDrawer: InlineScriptEditorDrawer
</script>

<InlineScriptEditorDrawer bind:this={inlineScriptEditorDrawer} />

<SplitPanesWrapper>
	<Pane size={25}>
		<PanelSection title="Inline scripts">
			<div class="flex flex-col gap-2 w-full">
				{#if $app.inlineScripts && Object.keys($app.inlineScripts).length > 0}
					<div class="flex gap-2 flex-col ">
						{#each $app.inlineScripts ? Object.entries($app.inlineScripts) : [] as [key, value], index}
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<div
								class="{classNames(
									'border flex justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
									selectedScript?.path === value?.path ? 'bg-blue-100 text-blue-600' : ''
								)},"
								on:click={() => {
									selectedScript = value
									selectedScriptName = key
								}}
							>
								<span class="text-xs">{key}</span>
								{#if scriptsUsedByComponents.get(key)}
									<Badge color="dark-indigo">{scriptsUsedByComponents.get(key)}</Badge>
								{:else}
									<Badge color="red">Unused</Badge>
								{/if}
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-sm text-gray-500">No inline scripts</div>
				{/if}
			</div>
		</PanelSection>
	</Pane>
	<Pane size={75}>
		{#key selectedScriptName}
			{#if selectedScriptName}
				{#if selectedScript}
					<div class="h-full p-4 flex flex-col gap-2 ">
						<div class="flex w-full flex-row-reverse gap-2 items-center">
							<Button
								size="xs"
								color="light"
								variant="border"
								on:click={() => {
									if (selectedScriptName) {
										inlineScriptEditorDrawer?.openDrawer(selectedScriptName)
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
								on:click={deleteInlineScript}
							/>
						</div>

						<div class="border h-full">
							<SimpleEditor
								class="flex flex-1 grow h-full"
								lang="typescript"
								bind:code={selectedScript.content}
								fixedOverflowWidgets={false}
							/>
						</div>
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
			{:else}
				<div class="flex flex-col p-4 gap-2 text-sm"> Please choose a script </div>
			{/if}
		{/key}
	</Pane>
</SplitPanesWrapper>
