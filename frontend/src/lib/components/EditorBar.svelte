<script context="module" lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1044
</script>

<script lang="ts">
	import { ResourceService, VariableService, type Script } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import type Editor from './Editor.svelte'
	import ItemPicker from './ItemPicker.svelte'
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

	import {
		DiffIcon,
		DollarSign,
		History,
		Library,
		Link,
		Package,
		Plus,
		RotateCw,
		Save,
		Users
	} from 'lucide-svelte'
	import { capitalize, toCamel } from '$lib/utils'
	import type { Schema, SchemaProperty, SupportedLanguage } from '$lib/common'
	import ScriptVersionHistory from './ScriptVersionHistory.svelte'
	import ScriptGen from './copilot/ScriptGen.svelte'
	import type DiffEditor from './DiffEditor.svelte'
	import { getResetCode } from '$lib/script_helpers'
	import Popover from './Popover.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'
	import type { EditorBarUi } from './custom_ui'
	import EditorSettings from './EditorSettings.svelte'
	import { quicktype, InputData, JSONSchemaInput, FetchingJSONSchemaStore } from 'quicktype-core'

	export let lang: SupportedLanguage | 'bunnative' | undefined
	export let editor: Editor | undefined
	export let websocketAlive: {
		pyright: boolean
		ruff: boolean
		deno: boolean
		go: boolean
		shellcheck: boolean
	}
	export let iconOnly: boolean = false
	export let validCode: boolean = true
	export let kind: 'script' | 'trigger' | 'approval' = 'script'
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative' =
		'script'
	export let collabMode = false
	export let collabLive = false
	export let collabUsers: { name: string }[] = []
	export let scriptPath: string | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let args: Record<string, any>
	export let noHistory = false
	export let saveToWorkspace = false
	export let customUi: EditorBarUi = {}
	export let lastDeployedCode: string | undefined = undefined
	export let diffMode: boolean = false
	export let showHistoryDrawer: boolean = false

	let contextualVariablePicker: ItemPicker
	let variablePicker: ItemPicker
	let resourcePicker: ItemPicker
	let resourceTypePicker: ItemPicker
	let variableEditor: VariableEditor
	let resourceEditor: ResourceEditorDrawer
	let showContextVarPicker = false
	let showVarPicker = false
	let showResourcePicker = false
	let showResourceTypePicker = false

	$: showContextVarPicker = [
		'python3',
		'bash',
		'powershell',
		'go',
		'deno',
		'bun',
		'bunnative',
		'nativets',
		'php',
		'rust',
		'csharp',
		'nu',
		'java'
		// for related places search: ADD_NEW_LANG
	].includes(lang ?? '')
	$: showVarPicker = [
		'python3',
		'bash',
		'powershell',
		'go',
		'deno',
		'bun',
		'bunnative',
		'nativets',
		'php',
		'rust',
		'csharp',
		'nu',
		'java'
		// for related places search: ADD_NEW_LANG
	].includes(lang ?? '')
	$: showResourcePicker = [
		'python3',
		'bash',
		'powershell',
		'go',
		'deno',
		'bun',
		'bunnative',
		'nativets',
		'php',
		'rust',
		'csharp',
		'nu',
		'java'
		// for related places search: ADD_NEW_LANG
	].includes(lang ?? '')
	$: showResourceTypePicker =
		['typescript', 'javascript'].includes(scriptLangToEditorLang(lang)) ||
		lang === 'python3' ||
		lang === 'php' ||
		lang === 'rust'

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

	async function quicktypeJSONSchema(targetLanguage, typeName, jsonSchemaString, rendererOptions) {
		const schemaInput = new JSONSchemaInput(new FetchingJSONSchemaStore())

		// We could add multiple schemas for multiple types,
		// but here we're just making one type from JSON schema.
		await schemaInput.addSource({ name: typeName, schema: jsonSchemaString })

		const inputData = new InputData()
		inputData.addInput(schemaInput)

		return await quicktype({
			inputData,
			lang: targetLanguage,
			rendererOptions
		})
	}
	async function resourceTypePickCallback(name: string) {
		if (!editor) return
		const resourceType = await ResourceService.getResourceType({
			workspace: $workspaceStore ?? 'NO_W',
			path: name
		})

		if (lang == 'python3') {
			const pySchema = pythonCompile(resourceType.schema as any)

			editor.insertAtCursor(`class ${name}(TypedDict):\n${pySchema}\n`)
			const code = editor.getCode()
			if (!code.includes('from typing import TypedDict')) {
				editor.insertAtBeginning('from typing import TypedDict\n')
			}
		} else if (lang === 'php') {
			const phpSchema = phpCompile(resourceType.schema as any)
			const rtName = toCamel(capitalize(name))
			editor.insertAtCursor(`if (!class_exists('${rtName}')) {\nclass ${rtName} {\n${phpSchema}\n`)
			editor.backspace()
			editor.insertAtCursor('}')
		} else if (lang === 'rust') {
			const { lines: lines } = await quicktypeJSONSchema(
				'rust',
				name,
				JSON.stringify(resourceType.schema),
				{
					"leading-comments": false,
					"density": "dense",
					"derive-debug": true
				}
			)
			editor.insertAtCurrentLine(lines.join('\n'))
		} else {
			const tsSchema = compile(resourceType.schema as any)
			editor.insertAtCursor(`type ${toCamel(capitalize(name))} = ${tsSchema}`)
		}
		sendUserToast(`${name} inserted at cursor`)
	}

	function phpCompile(schema: Schema) {
		let res = '  '
		const entries = Object.entries(schema.properties)
		if (entries.length === 0) {
			return 'array'
		}
		let i = 0
		for (let [name, prop] of entries) {
			let typ = 'array'
			if (prop.type === 'array') {
				typ = 'array'
			} else if (prop.type === 'string') {
				typ = 'string'
			} else if (prop.type === 'number') {
				typ = 'float'
			} else if (prop.type === 'integer') {
				typ = 'int'
			} else if (prop.type === 'boolean') {
				typ = 'bool'
			}
			res += `public ${typ} $${name};`
			i++
			if (i < entries.length) {
				res += '\n'
			}
		}
		return res
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
			const resetCode = getResetCode(lang, kind as Script['kind'], template)
			editor.setCode(resetCode)
		}
	}

	function windmillPathToCamelCaseName(path: string): string {
		const parts = path.split('/')
		const lastPart = parts[parts.length - 1]

		const words = lastPart.split('_')

		return words
			.map((word, index) => {
				if (index === 0) {
					// Lowercase the first word
					return word.toLowerCase()
				} else {
					// Capitalize the first letter of subsequent words
					return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
				}
			})
			.join('')
	}
</script>

{#if scriptPath}
	<Drawer bind:open={showHistoryDrawer} size="1200px">
		<DrawerContent title="Versions History" on:close={() => (showHistoryDrawer = false)}>
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
		} else if (lang === 'bun' || lang === 'bunnative' || lang == 'nativets') {
			editor.insertAtCursor(`process.env["${name}"]`)
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
		} else if (lang == 'powershell') {
			editor.insertAtCursor(`$Env:${name}`)
		} else if (lang == 'php') {
			editor.insertAtCursor(`getenv('${name}');`)
		} else if (lang == 'rust') {
			editor.insertAtCursor(`std::env::var("${name}").unwrap();`)
		} else if (lang == 'csharp') {
			editor.insertAtCursor(`Environment.GetEnvironmentVariable("${name}");`)
		} else if (lang == 'nu') {
			editor.insertAtCursor(`$env.${name}`)
		} else if (lang == 'java') {
			editor.insertAtCursor(`System.getenv("${name}");`)
			// for related places search: ADD_NEW_LANG
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
		} else if (lang === 'bun' || lang === 'bunnative') {
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
		} else if (lang == 'powershell') {
			editor.insertAtCursor(`$Headers = @{\n"Authorization" = "Bearer $Env:WM_TOKEN"`)
			editor.arrowDown()
			editor.insertAtCursor(
				`\nInvoke-RestMethod -Headers $Headers -Uri "$Env:BASE_INTERNAL_URL/api/w/$Env:WM_WORKSPACE/variables/get_value/${path}"`
			)
		} else if (lang == 'nativets') {
			const code = editor.getCode()
			if (!code.includes(`import * as wmill from`)) {
				editor.insertAtBeginning(`import * as wmill from "./windmill.ts"\n`)
			}
			editor.insertAtCursor(`(await wmill.getVariable('${path}'))`)
		} else if (lang == 'php') {
			editor.insertAtCursor(`$ch = curl_init(getenv('BASE_INTERNAL_URL') . '/api/w/' . getenv('WM_WORKSPACE') . '/variables/get_value/${path}');
curl_setopt($ch, CURLOPT_HTTPHEADER, array('Authorization: Bearer ' . getenv('WM_TOKEN')));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$var = json_decode(curl_exec($ch));`)
		} else if (lang == 'csharp') {
			editor.insertAtCursor(`var baseUrl = Environment.GetEnvironmentVariable("BASE_INTERNAL_URL");
var workspace = Environment.GetEnvironmentVariable("WM_WORKSPACE");
var uri = $"{baseUrl}/api/w/{workspace}/variables/get_value/${path}";
using var client = new HttpClient();
client.DefaultRequestHeaders.Add("Authorization", $"Bearer {Environment.GetEnvironmentVariable("WM_TOKEN")}");

string ${windmillPathToCamelCaseName(path)} = await client.GetStringAsync(uri);
`)
		} else if (lang == 'nu') {
			editor.insertAtCursor(`get_variable ${path}`)
		} else if (lang == 'java') {
			editor.insertAtCursor(`(Wmill.getVariable("${path}"))`)
			// for related places search: ADD_NEW_LANG
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
			New variable
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
		} else if (lang === 'bun' || lang === 'bunnative') {
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
		} else if (lang == 'powershell') {
			editor.insertAtCursor(`$Headers = @{\n"Authorization" = "Bearer $Env:WM_TOKEN"`)
			editor.arrowDown()
			editor.insertAtCursor(
				`\nInvoke-RestMethod -Headers $Headers -Uri "$Env:BASE_INTERNAL_URL/api/w/$Env:WM_WORKSPACE/resources/get_value_interpolated/${path}"`
			)
		} else if (lang == 'nativets') {
			const code = editor.getCode()
			if (!code.includes(`import * as wmill from`)) {
				editor.insertAtBeginning(`import * as wmill from "./windmill.ts"\n`)
			}
			editor.insertAtCursor(`(await wmill.getResource('${path}'))`)
		} else if (lang == 'php') {
			editor.insertAtCursor(`$ch = curl_init(getenv('BASE_INTERNAL_URL') . '/api/w/' . getenv('WM_WORKSPACE') . '/resources/get_value_interpolated/${path}');
curl_setopt($ch, CURLOPT_HTTPHEADER, array('Authorization: Bearer ' . getenv('WM_TOKEN')));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$res = json_decode(curl_exec($ch));`)
		} else if (lang == 'csharp') {
			if (!editor.getCode().includes(`using System.Text.Json.Nodes;`)) {
				editor.insertAtBeginning(`using System.Text.Json.Nodes;\n`)
			}
			editor.insertAtCursor(`var baseUrl = Environment.GetEnvironmentVariable("BASE_INTERNAL_URL");
var workspace = Environment.GetEnvironmentVariable("WM_WORKSPACE");
var uri = $"{baseUrl}/api/w/{workspace}/resources/get_value_interpolated/${path}";
using var client = new HttpClient();
client.DefaultRequestHeaders.Add("Authorization", $"Bearer {Environment.GetEnvironmentVariable("WM_TOKEN")}");

JsonNode ${windmillPathToCamelCaseName(path)} = JsonNode.Parse(await client.GetStringAsync(uri));
`)
		} else if (lang == 'nu') {
			editor.insertAtCursor(`get_resource ${path}`)
		} else if (lang == 'java') {
			editor.insertAtCursor(`(Wmill.getResource("${path}"))`)
			// for related places search: ADD_NEW_LANG
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
			href="{base}/resources?connect_app=undefined"
		>
			Add resource
		</Button>
	</div>
</ItemPicker>

{#if showResourceTypePicker}
	<ItemPicker
		bind:this={resourceTypePicker}
		pickCallback={async (_, name) => {
			resourceTypePickCallback(name)
		}}
		tooltip="Resources Types are the schemas associated with a Resource. They define the structure of the data that is returned from a Resource."
		documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
		itemName="Resource Type"
		extraField="name"
		loadItems={async () =>
			await ResourceService.listResourceType({ workspace: $workspaceStore ?? 'NO_W' })}
	/>
{/if}
<ResourceEditorDrawer bind:this={resourceEditor} on:refresh={resourcePicker.openDrawer} />
<VariableEditor bind:this={variableEditor} on:create={variablePicker.openDrawer} />

<div class="flex justify-between items-center overflow-y-auto w-full p-0.5">
	<div class="flex items-center">
		<div
			title={validCode ? 'Main function parsable' : 'Main function not parsable'}
			class="rounded-full w-2 h-2 mx-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
		></div>
		<div class="flex items-center gap-0.5">
			{#if showContextVarPicker && customUi?.contextVar != false}
				<Button
					title="Add context variable"
					color="light"
					on:click={contextualVariablePicker.openDrawer}
					size="xs"
					btnClasses="!font-medium text-tertiary"
					spacingSize="md"
					startIcon={{ icon: DollarSign }}
					{iconOnly}
					>+Context var
				</Button>
			{/if}
			{#if showVarPicker && customUi?.variable != false}
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

			{#if showResourcePicker && customUi?.resource != false}
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

			{#if showResourceTypePicker && customUi?.type != false}
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

			{#if customUi?.reset != false}
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
			{/if}

			{#if customUi?.assistants != false}
				{#if lang == 'deno' || lang == 'python3' || lang == 'go' || lang == 'bash' || lang == 'nu'}
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
						<span class="-my-1">
							{#if lang == 'deno'}
								(<span class={websocketAlive.deno ? 'green' : 'text-red-700'}>Deno</span>)
							{:else if lang == 'go'}
								(<span class={websocketAlive.go ? 'green' : 'text-red-700'}>Go</span>)
							{:else if lang == 'python3'}
								(<span class={websocketAlive.pyright ? 'green' : 'text-red-700'}>Pyright</span>
								<span class={websocketAlive.ruff ? 'green' : 'text-red-700'}>Ruff</span>)
							{:else if lang == 'bash'}
								(<span class={websocketAlive.shellcheck ? 'green' : 'text-red-700'}>Shellcheck</span
								>)
							{/if}
						</span>
					</Button>
				{/if}
			{/if}

			{#if customUi?.diffMode != false}
				<div class="flex items-center px-3">
					<Toggle
						options={{ right: '' }}
						size="xs"
						checked={diffMode}
						disabled={!lastDeployedCode}
						on:change={(e) => {
							const turnOn = e.detail
							dispatch(turnOn ? 'showDiffMode' : 'hideDiffMode')
						}}
					/>
					<Popover>
						<svelte:fragment slot="text">Toggle diff mode</svelte:fragment>
						<DiffIcon class="ml-1 text-tertiary" size={14} />
					</Popover>
				</div>
			{/if}

			{#if collabMode && customUi?.multiplayer != false}
				<div class="flex items-center px-3">
					<Toggle
						options={{ right: '' }}
						size="xs"
						checked={collabLive}
						on:change={() => dispatch('toggleCollabMode')}
					/>
					<Popover>
						<svelte:fragment slot="text">Multiplayer</svelte:fragment>
						<Users class="ml-1 text-tertiary" size={14} />
					</Popover>
					{#if collabLive}
						<button
							title="Show invite link"
							class="p-1 rounded hover:bg-gray-400 mx-1 border"
							on:click={() => dispatch('collabPopup')}><Link size={14} /></button
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

			{#if customUi?.aiGen != false}
				<ScriptGen {editor} {diffEditor} {lang} {iconOnly} {args} />
			{/if}

			<EditorSettings {customUi} />
		</div>
	</div>

	<div class="flex flex-row items-center gap-2">
		<slot name="right" />
		{#if scriptPath && !noHistory}
			<Button
				btnClasses="!font-medium text-tertiary"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={() => (showHistoryDrawer = true)}
				{iconOnly}
				startIcon={{ icon: History }}
				title="See history"
			>
				History
			</Button>
		{/if}
		{#if SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS && customUi?.library != false}
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
		{#if saveToWorkspace}
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: Save }}
				on:click={() => dispatch('createScriptFromInlineScript')}
				iconOnly={false}
			>
				Save to workspace
			</Button>
		{/if}
	</div>
</div>

<style lang="postcss">
	span.green {
		@apply text-green-600 animate-[pulse_5s_ease-in-out_infinite];
	}
</style>
