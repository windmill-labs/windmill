<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import FlowScriptPicker from '$lib/components/flows/pickers/FlowScriptPicker.svelte'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Script, type Preview } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { capitalize, emptySchema, getScriptByPath } from '$lib/utils'
	import { faCodeBranch } from '@fortawesome/free-solid-svg-icons'
	import { Building, Globe2 } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fly } from 'svelte/transition'
	import type { AppViewerContext } from '../../types'
	import { defaultCode } from '../component'
	import InlineScriptList from '../settingsPanel/mainInput/InlineScriptList.svelte'
	import WorkspaceScriptList from '../settingsPanel/mainInput/WorkspaceScriptList.svelte'

	export let name: string
	export let componentType: string | undefined = undefined

	let tab = 'inlinescripts'
	let filter: string = ''
	let picker: Drawer

	const { appPath, app } = getContext<AppViewerContext>('AppViewerContext')
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
		subkind: 'pgsql' | 'mysql' | 'fetch' | undefined = undefined
	) {
		const content =
			defaultCode(componentType ?? '', subkind || language) ??
			initialCode(language, Script.kind.SCRIPT, subkind ?? 'flow')

		return newInlineScript(content, language, path)
	}

	async function newInlineScript(content: string, language: Preview.language, path: string) {
		const fullPath = `${appPath}/${path}`

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

	async function pickScript(path: string) {
		const script = await getScriptByPath(path)
		newInlineScript(script.content, script.language, path)
	}

	async function pickHubScript(path: string) {
		const script = await getScriptByPath(path)
		newInlineScript(script.content, script.language, path)
	}

	function pickInlineScript(name: string) {
		const unusedInlineScriptIndex = $app.unusedInlineScripts?.findIndex(
			(script) => script.name === name
		)
		const unusedInlineScript = $app.unusedInlineScripts?.[unusedInlineScriptIndex]

		$app.unusedInlineScripts.splice(unusedInlineScriptIndex, 1)
		$app = $app
		dispatch('new', unusedInlineScript.inlineScript)
	}

	const langs = ['deno', 'python3', 'go', 'bash'] as Script.language[]
</script>

<Drawer bind:this={picker} size="1000px">
	<DrawerContent title="Script/Flow Picker" on:close={picker.closeDrawer}>
		<div>
			<div class="max-w-6xl">
				<Tabs bind:selected={tab}>
					<Tab size="sm" value="inlinescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Detached Inline Scripts
						</div>
					</Tab>
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
				<div class="my-2" />
				<div class="flex flex-col gap-y-16">
					<div class="flex flex-col">
						{#if tab == 'inlinescripts'}
							<InlineScriptList
								on:pick={(e) => pickInlineScript(e.detail)}
								inlineScripts={$app.unusedInlineScripts
									? $app.unusedInlineScripts.map((uis) => uis.name)
									: []}
							/>
						{:else if tab == 'workspacescripts'}
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

<div class="flex flex-col px-4 gap-2 text-sm" in:fly={{ duration: 50 }}>
	<div class="mt-4 flex justify-between gap-8 mb-2">
		<div class="font-bold items-baseline">Choose a language:</div>
		<div class="flex gap-2">
			<Button
				on:click={() => picker?.openDrawer()}
				size="xs"
				color="blue"
				startIcon={{ icon: faCodeBranch }}
				btnClasses="truncate"
			>
				Fork a workspace or hub script
			</Button>
			<Button
				on:click={() => dispatch('delete')}
				size="xs"
				color="red"
				variant="border"
				btnClasses="truncate"
			>
				Cancel
			</Button>
		</div>
	</div>

	<div class="flex flex-row w-full justify-between">
		<div class="">
			<div class="mb-1 text-sm font-semibold">Backend</div>

			<div class="flex gap-2 flex-row flex-wrap">
				{#each langs as lang}
					<FlowScriptPicker
						label={lang === 'deno' ? 'Typescript' : capitalize(lang)}
						{lang}
						on:click={() => {
							createInlineScriptByLanguage(lang, name)
						}}
					/>
				{/each}
			</div>

			<div class="mt-4 mb-2 text-sm">Typescript templates</div>

			<div class="flex gap-2 flex-row flex-wrap mb-4">
				<FlowScriptPicker
					label={`PostgreSQL`}
					lang="pgsql"
					on:click={() => {
						createInlineScriptByLanguage(Script.language.DENO, name, 'pgsql')
					}}
				/>

				<FlowScriptPicker
					label={`HTTP`}
					lang="fetch"
					on:click={() => {
						createInlineScriptByLanguage(Script.language.DENO, name, 'fetch')
					}}
				/>
			</div>
		</div>
		<div class="">
			<div class="mb-1 text-sm font-semibold">
				Frontend
				<Tooltip
					documentationLink="https://docs.windmill.dev/docs/apps/app-runnable#frontend-script"
				>
					Frontend scripts are executed in the browser and can manipulate the app context directly
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

// you can also navigate (goto), recompute a script (recompute), or set a tab (setTab)

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
	</div>
</div>
