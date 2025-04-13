<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { defaultScriptLanguages, getScriptByPath, processLangs } from '$lib/scripts'

	import { Building, GitFork, Globe2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fly } from 'svelte/transition'
	import { defaultCode } from '../component'
	import WorkspaceScriptList from '../settingsPanel/mainInput/WorkspaceScriptList.svelte'
	import RunnableSelector from '../settingsPanel/mainInput/RunnableSelector.svelte'
	import { defaultScripts } from '$lib/stores'
	import DefaultScripts from '$lib/components/DefaultScripts.svelte'
	import type { Preview } from '$lib/gen'
	import type { InlineScript } from '../../types'

	export let componentType: string | undefined = undefined
	export let showScriptPicker = false
	export let rawApps = false
	export let unusedInlineScripts: { name: string; inlineScript: InlineScript }[]

	let tab = 'workspacescripts'
	let filter: string = ''
	let picker: Drawer

	const dispatch = createEventDispatcher()

	async function inferInlineScriptSchema(
		language: Preview['language'],
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
		language: Preview['language'],
		subkind: 'pgsql' | 'mysql' | 'fetch' | undefined = undefined
	) {
		const content =
			defaultCode(componentType ?? '', (subkind || language) ?? '') ??
			initialCode(language, 'script', subkind ?? 'flow')

		return newInlineScript(content, language)
	}

	async function newInlineScript(content: string, language: Preview['language']) {
		let schema: Schema = emptySchema()

		schema = await inferInlineScriptSchema(language, content, schema)
		const newInlineScript = {
			content,
			language,
			schema
		}
		dispatch('new', newInlineScript)
	}

	async function pickScript(path: string) {
		const script = await getScriptByPath(path)
		newInlineScript(script.content, script.language)
	}

	async function pickHubScript(path: string) {
		const script = await getScriptByPath(path)
		newInlineScript(script.content, script.language)
	}

	$: langs = processLangs(undefined, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
		.map((l) => [defaultScriptLanguages[l], l])
		.filter(
			(x) =>
				x[1] != 'docker' &&
				($defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1]))
		) as [string, Preview['language']][]
</script>

<Drawer bind:this={picker} size="1000px">
	<DrawerContent title="Script/Flow Picker" on:close={picker.closeDrawer}>
		<div>
			<div class="max-w-6xl">
				<Tabs bind:selected={tab}>
					<Tab size="sm" value="workspacescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Scripts
						</div>
					</Tab>

					<Tab size="sm" value="hubscripts">
						<div class="flex gap-2 items-center my-1">
							<Globe2 size={18} />
							Hub Scripts
						</div>
					</Tab>
				</Tabs>
				<div class="my-2"></div>
				<div class="flex flex-col gap-y-16">
					<div class="flex flex-col">
						{#if tab == 'workspacescripts'}
							<WorkspaceScriptList on:pick={(e) => pickScript(e.detail)} />
						{:else if tab == 'hubscripts'}
							<PickHubScript bind:filter on:pick={(e) => pickHubScript(e.detail.path)} />
						{/if}
					</div>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>

<div
	class="flex flex-col px-4 gap-2 text-sm"
	in:fly={{ duration: 50 }}
	id="app-editor-empty-runnable"
>
	<div class="mt-2 flex justify-between gap-4" id="app-editor-runnable-header">
		<div class="font-bold items-baseline truncate">Choose a language</div>
		<div class="flex gap-2">
			{#if showScriptPicker}
				<RunnableSelector {unusedInlineScripts} {rawApps} on:pick hideCreateScript />
			{/if}
			<Button
				on:click={() => picker?.openDrawer()}
				size="xs"
				variant="border"
				color="light"
				startIcon={{ icon: GitFork }}
				btnClasses="truncate"
			>
				Fork other script
			</Button>

			<Button
				on:click={() => dispatch('delete')}
				size="xs"
				color="red"
				variant="border"
				btnClasses="truncate"
			>
				Clear
			</Button>
		</div>
	</div>

	<div class="flex flex-row w-full gap-8">
		<div id="app-editor-backend-runnables">
			<div class="mb-1 text-sm font-semibold flex gap-4">Backend <DefaultScripts /> </div>

			<div class="flex flex-row flex-wrap gap-2">
				{#each langs as [label, lang] (lang)}
					<FlowScriptPicker
						{label}
						{lang}
						on:click={() => {
							createInlineScriptByLanguage(lang)
						}}
						id={`create-${lang}-script`}
					/>
				{/each}
			</div>
		</div>
		{#if !rawApps}
			<div id="app-editor-frontend-runnables">
				<div class="mb-1 text-sm font-semibold">
					Frontend
					<Tooltip
						documentationLink="https://www.windmill.dev/docs/apps/app-runnable-panel#frontend-scripts"
					>
						Frontend scripts are executed in the browser and can manipulate the app context
						directly.
					</Tooltip>
				</div>

				<div>
					<FlowScriptPicker
						label={`JavaScript`}
						lang="javascript"
						on:click={() => {
							const newInlineScript = {
								content: `// read outputs and ctx
console.log(ctx.email)

// access a global state store
if (!state.foo) { state.foo = 0 }
state.foo += 1

// for reactivity to work, you need to assign a value and not modify it in place
// e.g: state.foo.push(1) will not work but 'state.foo = [...state.foo, 1]' will.
// you may also just reassign as next statement 'state.foo = state.foo'

// you can also navigate (goto), recompute a script (recompute), or set a tab (setTab)
// Inputs and display components support settings their value directly
// setValue('a', "Bar") 
// Tables support setting their selected index (setSelectedIndex)
// all helpers can be found at https://www.windmill.dev/docs/apps/app-runnable-panel#frontend-scripts-helpers

return state.foo`,
								language: 'frontend',
								path: 'frontend script',
								schema: undefined
							}
							dispatch('new', newInlineScript)
						}}
					/>
				</div>
			</div>
		{/if}
	</div>
</div>
