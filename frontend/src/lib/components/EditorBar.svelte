<script context="module" lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1044
</script>

<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import type Editor from './Editor.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import ResourceEditor from './ResourceEditor.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import Button from './common/button/Button.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Drawer } from './common'
	import WorkspaceScriptPicker from './flows/pickers/WorkspaceScriptPicker.svelte'
	import PickHubScript from './flows/pickers/PickHubScript.svelte'
	import ToggleHubWorkspace from './ToggleHubWorkspace.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import { SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS } from '$lib/consts'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { getScriptByPath, scriptLangToEditorLang } from '$lib/scripts'
	import Toggle from './Toggle.svelte'
	import { DollarSign, History, Library, Link, Package, Plus, RotateCw, Users } from 'lucide-svelte'
	import { capitalize, toCamel } from '$lib/utils'
	import type { Schema, SchemaProperty, SupportedLanguage } from '$lib/common'
	import ScriptVersionHistory from './ScriptVersionHistory.svelte'
	import ScriptGen from './copilot/ScriptGen.svelte'
	import type DiffEditor from './DiffEditor.svelte'
	import { getResetCode } from '$lib/script_helpers'
	import type { Script } from '$lib/gen'

	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let websocketAlive: {
		pyright: boolean
		black: boolean
		ruff: boolean
		deno: boolean
		go: boolean
		shellcheck: boolean
		bun: boolean
	}
	export let iconOnly: boolean = false
	export let validCode: boolean = true
	export let kind: 'script' | 'trigger' | 'approval' = 'script'
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' = 'script'
	export let collabMode = false
	export let collabLive = false
	export let collabUsers: { name: string }[] = []
	export let scriptPath: string | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let args: Record<string, any>

	let contextualVariablePicker: ItemPicker
	let variablePicker: ItemPicker
	let resourcePicker: ItemPicker
	let resourceTypePicker: ItemPicker
	let variableEditor: VariableEditor
	let resourceEditor: ResourceEditor
	let showContextVarPicker = false
	let showVarPicker = false
	let showResourcePicker = false
	let showResourceTypePicker = false

	$: showContextVarPicker = ['python3', 'bash', 'go', 'deno', 'bun'].includes(lang)
	$: showVarPicker = ['python3', 'bash', 'go', 'deno', 'bun'].includes(lang)
	$: showResourcePicker = ['python3', 'bash', 'go', 'deno', 'bun'].includes(lang)
	$: showResourceTypePicker = scriptLangToEditorLang(lang) === 'typescript' || lang === 'python3'

	let codeViewer: Drawer
	let codeObj: { language: SupportedLanguage; content: string } | undefined = undefined

	function addEditorActions() {
		editor?.addAction('insert-variable', 'Windmill: Insert variable', () => {
			variablePicker.openDrawer()
		})
		editor?.addAction('insert-resource', 'Windmill: Insert resource', () => {
			resourcePicker.openDrawer()
		})
	}

	$: editor && addEditorActions()

	async function loadVariables() {
		return await VariableService.listVariable({ workspace: $workspaceStore ?? '' })
	}

	async function loadContextualVariables() {
		return await VariableService.listContextualVariables({
			workspace: $workspaceStore ?? 'NO_W'
		})
	}

	let scriptPicker: Drawer
	let pick_existing: 'hub' | 'workspace' = 'hub'
	let filter = ''

	async function onScriptPick(e: { detail: { path: string } }) {
		codeObj = undefined
		codeViewer?.openDrawer?.()
		codeObj = await getScriptByPath(e.detail.path ?? '')
	}

	const dispatch = createEventDispatcher()

	function compile(schema: Schema) {
		function rec(x: { [name: string]: SchemaProperty }, root = false) {
			let res = '{\n'
			const entries = Object.entries(x)
			if (entries.length == 0) {
				return 'any'
			}
			let i = 0
			for (let [name, prop] of entries) {
				if (prop.type == 'object') {
					res += `${name}: ${rec(prop.properties ?? {})}`
				} else if (prop.type == 'array') {
					res += `${name}: ${prop?.items?.type ?? 'any'}[]`
				} else {
					let typ = prop?.type ?? 'any'
					if (typ == 'integer') {
						typ = 'number'
					}
					res += `${name}: ${typ}`
				}
				i++
				if (i < entries.length) {
					res += ',\n'
				}
			}
			if (!root) {
				res += '\n}'
			}
			return res
		}
		return rec(schema.properties, true)
	}

	function pythonCompile(schema: Schema) {
		let res = ''
		const entries = Object.entries(schema.properties)
		if (entries.length === 0) {
			return 'dict'
		}
		let i = 0
		for (let [name, prop] of entries) {
			let typ = 'dict'
			if (prop.type === 'array') {
				typ = 'list'
			} else if (prop.type === 'string') {
				typ = 'str'
			} else if (prop.type === 'number') {
				typ = 'float'
			} else if (prop.type === 'integer') {
				typ = 'int'
			} else if (prop.type === 'boolean') {
				typ = 'bool'
			}
			res += `${name}: ${typ}`
			i++
			if (i < entries.length) {
				res += '\n'
			}
		}
		return res
	}

	function clearContent() {
		if (editor) {
			const resetCode = getResetCode(lang, kind as Script.kind, template)
			editor.setCode(resetCode)
		}
	}

	let historyBrowserDrawerOpen = false
</script>

{#if scriptPath}
	<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
		<DrawerContent title="Versions History" on:close={() => (historyBrowserDrawerOpen = false)}>
			<ScriptVersionHistory {scriptPath} />
		</DrawerContent>
	</Drawer>
{/if}

<Drawer bind:this={scriptPicker} size="900px">
	<DrawerContent title="Code" on:close={scriptPicker.closeDrawer}>
		{#if pick_existing == 'hub'}
			<PickHubScript bind:filter {kind} on:pick={onScriptPick}>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</PickHubScript>
		{:else}
			<WorkspaceScriptPicker bind:filter {kind} on:pick={onScriptPick}>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</WorkspaceScriptPicker>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={codeViewer} size="600px">
	<DrawerContent title="Code" on:close={codeViewer.closeDrawer}>
		{#if codeObj}
			<HighlightCode language={codeObj?.language} code={codeObj?.content} />
		{:else}
			<Skeleton layout={[[40]]} />
		{/if}
	</DrawerContent>
</Drawer>
<ItemPicker
	bind:this={contextualVariablePicker}
	pickCallback={(path, name) => {
		if (!editor) return
		if (lang == 'deno') {
			editor.insertAtCursor(`Deno.env.get('${name}')`)
		} else if (lang === 'bun') {
			editor.insertAtCursor(`Bun.env["${name}"]`)
		} else if (lang == 'python3') {
			if (!editor.getCode().includes('import os')) {
				editor.insertAtBeginning('import os\n')
			}
			editor.insertAtCursor(`os.environ.get("${name}")`)
		} else if (lang == 'go') {
			if (!editor.getCode().includes('"os"')) {
				editor.insertAtLine('import "os"\n', 2)
			}
			editor.insertAtCursor(`os.Getenv("${name}")`)
		} else if (lang == 'bash') {
			editor.insertAtCursor(`$${name}`)
		}
		sendUserToast(`${name} inserted at cursor`)
	}}
	tooltip="Contextual Variables are variables whose values are contextual to the Script
	execution. They are are automatically set by Windmill."
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets#contextual-variables"
	itemName="Contextual Variable"
	extraField="name"
	loadItems={loadContextualVariables}
/>

<ItemPicker
	bind:this={variablePicker}
	pickCallback={(path, name) => {
		if (!editor) return
		if (lang == 'deno') {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(`import * as wmill from "npm:windmill-client@1"\n`)
			}
			editor.insertAtCursor(`(await wmill.getVariable('${path}'))`)
		} else if (lang === 'bun') {
			const code = editor.getCode()
			if (!code.includes(`import * as wmill from`)) {
				editor.insertAtBeginning(`import * as wmill from "windmill-client"\n`)
			}
			editor.insertAtCursor(`(await wmill.getVariable('${path}'))`)
		} else if (lang == 'python3') {
			if (!editor.getCode().includes('import wmill')) {
				editor.insertAtBeginning('import wmill\n')
			}
			editor.insertAtCursor(`wmill.get_variable("${path}")`)
		} else if (lang == 'go') {
			if (!editor.getCode().includes('wmill "github.com/windmill-labs/windmill-go-client"')) {
				editor.insertAtLine('import wmill "github.com/windmill-labs/windmill-go-client"\n\n', 3)
			}
			editor.insertAtCursor(`v, _ := wmill.GetVariable("${path}")`)
		} else if (lang == 'bash') {
			editor.insertAtCursor(`curl -s -H "Authorization: Bearer $WM_TOKEN" \\
  "$BASE_INTERNAL_URL/api/w/$WM_WORKSPACE/variables/get_value/${path}" | jq -r .`)
		}
		sendUserToast(`${name} inserted at cursor`)
	}}
	tooltip="Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow."
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	itemName="Variable"
	extraField="path"
	loadItems={loadVariables}
	buttons={{ 'Edit/View': (x) => variableEditor.editVariable(x) }}
>
	<div slot="submission" class="flex flex-row">
		<Button
			variant="border"
			color="blue"
			size="sm"
			startIcon={{ icon: Plus }}
			on:click={() => {
				variableEditor.initNew()
			}}
		>
			New Variable
		</Button>
	</div>
</ItemPicker>

<ItemPicker
	bind:this={resourcePicker}
	pickCallback={(path, _) => {
		if (!editor) return
		if (lang == 'deno') {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(`import * as wmill from "npm:windmill-client@1"\n`)
			}
			editor.insertAtCursor(`(await wmill.getResource('${path}'))`)
		} else if (lang === 'bun') {
			const code = editor.getCode()
			if (!code.includes(`import * as wmill from`)) {
				editor.insertAtBeginning(`import * as wmill from "windmill-client"\n`)
			}
			editor.insertAtCursor(`(await wmill.getResource('${path}'))`)
		} else if (lang == 'python3') {
			if (!editor.getCode().includes('import wmill')) {
				editor.insertAtBeginning('import wmill\n')
			}
			editor.insertAtCursor(`wmill.get_resource("${path}")`)
		} else if (lang == 'go') {
			if (!editor.getCode().includes('wmill "github.com/windmill-labs/windmill-go-client"')) {
				editor.insertAtLine('import wmill "github.com/windmill-labs/windmill-go-client"\n\n', 3)
			}
			editor.insertAtCursor(`r, _ := wmill.GetResource("${path}")`)
		} else if (lang == 'bash') {
			editor.insertAtCursor(`curl -s -H "Authorization: Bearer $WM_TOKEN" \\
  "$BASE_INTERNAL_URL/api/w/$WM_WORKSPACE/resources/get_value_interpolated/${path}" | jq`)
		}
		sendUserToast(`${path} inserted at cursor`)
	}}
	tooltip="Resources represent connections to third party systems. Resources are a good way to define a connection to a frequently used third party system such as a database."
	documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
	itemName="Resource"
	buttons={{ 'Edit/View': (x) => resourceEditor.initEdit(x) }}
	extraField="description"
	extraField2="resource_type"
	loadItems={async () =>
		await ResourceService.listResource({ workspace: $workspaceStore ?? 'NO_W' })}
>
	<div slot="submission" class="flex flex-row gap-x-1 mr-2">
		<Button
			startIcon={{ icon: Plus }}
			target="_blank"
			variant="border"
			color="blue"
			size="sm"
			href="/resources?connect_app=undefined"
		>
			Add Resource
		</Button>
	</div>
</ItemPicker>

{#if showResourceTypePicker}
	<ItemPicker
		bind:this={resourceTypePicker}
		pickCallback={async (_, name) => {
			if (!editor) return
			const resourceType = await ResourceService.getResourceType({
				workspace: $workspaceStore ?? 'NO_W',
				path: name
			})

			if (lang == 'python3') {
				const pySchema = pythonCompile(resourceType.schema)

				editor.insertAtCursor(`class ${name}(TypedDict):\n${pySchema}\n`)
				const code = editor.getCode()
				if (!code.includes('from typing import TypedDict')) {
					editor.insertAtBeginning('from typing import TypedDict\n')
				}
			} else {
				const tsSchema = compile(resourceType.schema)
				editor.insertAtCursor(`type ${toCamel(capitalize(name))} = ${tsSchema}\n`)
			}
			sendUserToast(`${name} inserted at cursor`)
		}}
		tooltip="Resources Types are the schemas associated with a Resource. They define the structure of the data that is returned from a Resource."
		documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
		itemName="Resource Type"
		extraField="name"
		loadItems={async () =>
			await ResourceService.listResourceType({ workspace: $workspaceStore ?? 'NO_W' })}
	/>
{/if}
<ResourceEditor bind:this={resourceEditor} on:refresh={resourcePicker.openDrawer} />
<VariableEditor bind:this={variableEditor} on:create={variablePicker.openDrawer} />

<div class="flex justify-between items-center overflow-y-auto w-full p-0.5">
	<div class="flex items-center">
		<div
			title={validCode ? 'Main function parsable' : 'Main function not parsable'}
			class="rounded-full w-2 h-2 mx-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
		/>
		<div class="flex items-center gap-0.5">
			{#if showContextVarPicker}
				<Button
					title="Add context variable"
					color="light"
					on:click={contextualVariablePicker.openDrawer}
					size="xs"
					btnClasses="!font-medium text-tertiary"
					spacingSize="md"
					startIcon={{ icon: DollarSign }}
					{iconOnly}
					>+Context Var
				</Button>
			{/if}
			{#if showVarPicker}
				<Button
					title="Add variable"
					color="light"
					btnClasses="!font-medium text-tertiary"
					on:click={variablePicker.openDrawer}
					size="xs"
					spacingSize="md"
					startIcon={{ icon: DollarSign }}
					{iconOnly}
				>
					+Variable
				</Button>
			{/if}

			{#if showResourcePicker}
				<Button
					title="Add resource"
					btnClasses="!font-medium text-tertiary"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={resourcePicker.openDrawer}
					{iconOnly}
					startIcon={{ icon: Package }}
				>
					+Resource
				</Button>
			{/if}

			{#if showResourceTypePicker}
				<Button
					title="Add resource type"
					btnClasses="!font-medium text-tertiary"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={resourceTypePicker.openDrawer}
					{iconOnly}
					startIcon={{ icon: Package }}
				>
					+Type
				</Button>
			{/if}

			<Button
				title="Reset Content"
				btnClasses="!font-medium text-tertiary"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={clearContent}
				{iconOnly}
				startIcon={{ icon: RotateCw }}
			>
				Reset
			</Button>

			<Button
				btnClasses="!font-medium text-tertiary"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={() => editor?.reloadWebsocket()}
				startIcon={{
					icon: RotateCw,
					classes: websocketAlive[lang] == false ? 'animate-spin' : ''
				}}
				title="Reload assistants"
			>
				{#if !iconOnly}
					Assistants
				{/if}
				<span class="ml-1 -my-1">
					{#if lang == 'deno'}
						(<span class={websocketAlive.deno ? 'green' : 'text-red-700'}>Deno</span>)
					{:else if lang == 'bun'}
						(<span class={websocketAlive.bun ? 'green' : 'text-red-700'}>Bun</span>)
					{:else if lang == 'go'}
						(<span class={websocketAlive.go ? 'green' : 'text-red-700'}>Go</span>)
					{:else if lang == 'python3'}
						(<span class={websocketAlive.pyright ? 'green' : 'text-red-700'}>Pyright</span>
						<span class={websocketAlive.black ? 'green' : 'text-red-700'}>Black</span>
						<span class={websocketAlive.ruff ? 'green' : 'text-red-700'}>Ruff</span>)
					{:else if lang == 'bash'}
						(<span class={websocketAlive.shellcheck ? 'green' : 'text-red-700'}>Shellcheck</span>)
					{/if}
				</span>
			</Button>
			{#if collabMode}
				<div class="flex items-center px-1">
					<Toggle
						options={{ right: iconOnly ? '' : 'Multiplayer' }}
						size="xs"
						checked={collabLive}
						on:change={() => dispatch('toggleCollabMode')}
					/>
					{#if iconOnly}
						<Users class="ml-1" size={12} />
					{/if}
					{#if collabLive}
						<button
							title="Show invite link"
							class="p-1 rounded hover:bg-gray-400 mx-1 border"
							on:click={() => dispatch('collabPopup')}><Link size={12} /></button
						>
						<div class="isolate flex -space-x-2 pl-2">
							{#each collabUsers as user}
								<span
									class="inline-flex h-6 w-6 items-center justify-center rounded-full ring-2 ring-white bg-gray-600"
									title={user.name}
								>
									<span class="text-sm font-medium leading-none text-white"
										>{user.name.substring(0, 2).toLocaleUpperCase()}</span
									>
								</span>
							{/each}
						</div>
					{/if}
				</div>
			{/if}

			<ScriptGen {editor} {diffEditor} {lang} {iconOnly} {args} />

			<!-- <Popover
				notClickable
				placement="bottom"
				disappearTimeout={0}
				class="px-1"
				disablePopup={!iconOnly}
			>
				<Button
					btnClasses="!font-medium"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={editor?.format}
					{iconOnly}
					startIcon={{ icon: faBroom }}
				>
					Format ({getModifierKey()}+S)
				</Button>
				<svelte:fragment slot="text">
					Format <Kbd class="!text-gray-800">{getModifierKey()}</Kbd> + <Kbd class="!text-gray-800">
						S
					</Kbd>
				</svelte:fragment>
			</Popover> -->
		</div>
	</div>

	<div class="flex flex-row items-center gap-2">
		{#if scriptPath}
			<Button
				btnClasses="!font-medium text-tertiary"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={() => (historyBrowserDrawerOpen = true)}
				{iconOnly}
				startIcon={{ icon: History }}
				title="See history"
			>
				History
			</Button>
		{/if}
		{#if SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS}
			<Button
				btnClasses="!font-medium text-tertiary"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={scriptPicker.openDrawer}
				{iconOnly}
				startIcon={{ icon: Library }}
				title="Explore other scripts"
			>
				Library
			</Button>
		{/if}
	</div>
</div>

<style lang="postcss">
	span.green {
		@apply text-green-600 animate-[pulse_5s_ease-in-out_infinite];
	}
</style>
