<script module lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1044
</script>

<script lang="ts">
	import { run } from 'svelte/legacy'

	import { ResourceService, VariableService, WorkspaceService, type Script } from '$lib/gen'

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
	import { createEventDispatcher, untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { getScriptByPath, scriptLangToEditorLang } from '$lib/scripts'
	import Toggle from './Toggle.svelte'

	import {
		DatabaseIcon,
		DiffIcon,
		DollarSign,
		File,
		History,
		Library,
		Link,
		Package,
		Plus,
		RotateCw,
		Save,
		Users
	} from 'lucide-svelte'
	import { capitalize, formatS3Object, toCamel } from '$lib/utils'
	import type { Schema, SchemaProperty, SupportedLanguage } from '$lib/common'
	import ScriptVersionHistory from './ScriptVersionHistory.svelte'
	import ScriptGen from './copilot/ScriptGen.svelte'
	import type DiffEditor from './DiffEditor.svelte'
	import { getResetCode } from '$lib/script_helpers'
	import Popover from './Popover.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'
	import type { EditorBarUi } from './custom_ui'
	import EditorSettings from './EditorSettings.svelte'
	import S3FilePicker from './S3FilePicker.svelte'
	import DucklakeIcon from './icons/DucklakeIcon.svelte'

	interface Props {
		lang: SupportedLanguage | 'bunnative' | undefined
		editor: Editor | undefined
		websocketAlive: {
			pyright: boolean
			ruff: boolean
			deno: boolean
			go: boolean
			shellcheck: boolean
		}
		iconOnly?: boolean
		validCode?: boolean
		kind?: 'script' | 'trigger' | 'approval'
		template?: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative'
		collabMode?: boolean
		collabLive?: boolean
		collabUsers?: { name: string }[]
		scriptPath?: string | undefined
		diffEditor?: DiffEditor | undefined
		args: Record<string, any>
		noHistory?: boolean
		saveToWorkspace?: boolean
		customUi?: EditorBarUi
		lastDeployedCode?: string | undefined
		diffMode?: boolean
		showHistoryDrawer?: boolean
		right?: import('svelte').Snippet
		openAiChat?: boolean
	}

	let {
		lang,
		editor,
		websocketAlive,
		iconOnly = false,
		validCode = true,
		kind = 'script',
		template = 'script',
		collabMode = false,
		collabLive = false,
		collabUsers = [],
		scriptPath = undefined,
		diffEditor = undefined,
		args,
		noHistory = false,
		saveToWorkspace = false,
		customUi = {},
		lastDeployedCode = undefined,
		diffMode = false,
		showHistoryDrawer = $bindable(false),
		right,
		openAiChat = false
	}: Props = $props()

	let contextualVariablePicker: ItemPicker | undefined = $state()
	let variablePicker: ItemPicker | undefined = $state()
	let resourcePicker: ItemPicker | undefined = $state()
	let resourceTypePicker: ItemPicker | undefined = $state()
	let variableEditor: VariableEditor | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
	let ducklakePicker: ItemPicker | undefined = $state()
	let databasePicker: ItemPicker | undefined = $state()

	let showContextVarPicker = $derived(
		[
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
	)

	let showVarPicker = $derived(
		[
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
	)

	let showResourcePicker = $derived(
		[
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
	)

	let showS3Picker = $derived(
		['duckdb', 'python3'].includes(lang ?? '') ||
			['typescript', 'javascript'].includes(scriptLangToEditorLang(lang))
	)
	let showDucklakePicker = $derived(['duckdb'].includes(lang ?? ''))
	let showDatabasePicker = $derived(['duckdb'].includes(lang ?? ''))

	let showResourceTypePicker = $derived(
		['typescript', 'javascript'].includes(scriptLangToEditorLang(lang)) ||
			lang === 'python3' ||
			lang === 'php'
	)

	let codeViewer: Drawer | undefined = $state()
	let codeObj: { language: SupportedLanguage; content: string } | undefined = $state(undefined)

	function addEditorActions() {
		editor?.addAction('insert-variable', 'Windmill: Insert variable', () => {
			variablePicker?.openDrawer()
		})
		editor?.addAction('insert-resource', 'Windmill: Insert resource', () => {
			resourcePicker?.openDrawer()
		})
	}

	run(() => {
		editor && untrack(() => addEditorActions())
	})

	async function loadVariables() {
		return await VariableService.listVariable({ workspace: $workspaceStore ?? '' })
	}

	async function loadContextualVariables() {
		return await VariableService.listContextualVariables({
			workspace: $workspaceStore ?? 'NO_W'
		})
	}

	let scriptPicker: Drawer | undefined = $state()
	let pick_existing: 'hub' | 'workspace' = $state('hub')
	let filter = $state('')

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
	buttons={{ 'Edit/View': (x) => variableEditor?.editVariable(x) }}
>
	{#snippet submission()}
		<div class="flex flex-row">
			<Button
				variant="border"
				color="blue"
				size="sm"
				startIcon={{ icon: Plus }}
				on:click={() => {
					variableEditor?.initNew()
				}}
			>
				New variable
			</Button>
		</div>
	{/snippet}
</ItemPicker>

<ItemPicker
	bind:this={resourcePicker}
	pickCallback={(path, _, resType) => {
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
		} else if (lang == 'duckdb') {
			let t = { postgresql: 'postgres', mysql: 'mysql', bigquery: 'bigquery' }[resType]
			if (!t) {
				sendUserToast(`Resource type ${resType} is not supported in DuckDB`, true)
				editor.insertAtCursor(`'res://${path}'`)
				return
			} else {
				editor.insertAtCursor(`ATTACH 'res://${path}' AS db (TYPE ${t});`)
			}
		}

		sendUserToast(`${path} inserted at cursor`)
	}}
	tooltip="Resources represent connections to third party systems. Resources are a good way to define a connection to a frequently used third party system such as a database."
	documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
	itemName="Resource"
	buttons={{ 'Edit/View': (x) => resourceEditor?.initEdit(x) }}
	extraField="description"
	extraField2="resource_type"
	loadItems={async () =>
		await ResourceService.listResource({ workspace: $workspaceStore ?? 'NO_W' })}
>
	{#snippet submission()}
		<div class="flex flex-row gap-x-1 mr-2">
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
	{/snippet}
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

{#if showDucklakePicker}
	<ItemPicker
		bind:this={ducklakePicker}
		pickCallback={async (_, name) => {
			const connStr = name == 'main' ? 'ducklake' : `ducklake://${name}`
			editor?.insertAtCursor(`ATTACH '${connStr}' AS dl; USE dl;\n`)
		}}
		tooltip="Attach a Ducklake in your DuckDB script. Ducklake allows you to manipulate large data on S3 blob files through a traditional SQL interface."
		documentationLink="https://www.windmill.dev/docs/core_concepts/ducklake"
		itemName="ducklake"
		loadItems={async () =>
			(await WorkspaceService.listDucklakes({ workspace: $workspaceStore ?? 'NO_W' })).map(
				(path) => ({ path })
			)}
	/>
{/if}

{#if showDatabasePicker}
	<ItemPicker
		bind:this={databasePicker}
		pickCallback={(path, _, resType) => {
			if (!editor) return
			if (lang == 'duckdb') {
				let t = { postgresql: 'postgres', mysql: 'mysql', bigquery: 'bigquery' }[resType]
				editor.insertAtCursor(`ATTACH 'res://${path}' AS db (TYPE ${t});`)
			}
			sendUserToast(`${path} inserted at cursor`)
		}}
		tooltip="Attach a database resource in your script. This allows you to query data from the database using SQL."
		documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
		itemName="Database"
		buttons={{ 'Edit/View': (x) => resourceEditor?.initEdit(x) }}
		extraField="description"
		extraField2="resource_type"
		loadItems={async () =>
			await ResourceService.listResource({
				workspace: $workspaceStore ?? 'NO_W',
				resourceType: 'postgresql,mysql,bigquery'
			})}
	></ItemPicker>
{/if}

<S3FilePicker
	bind:this={s3FilePicker}
	readOnlyMode={false}
	on:selectAndClose={(s3obj) => {
		let s = `'${formatS3Object(s3obj.detail)}'`
		if (lang === 'duckdb') {
			if (s3obj.detail?.s3.endsWith('.json')) s = `read_json(${s})`
			if (s3obj.detail?.s3.endsWith('.csv')) s = `read_csv(${s})`
			if (s3obj.detail?.s3.endsWith('.parquet')) s = `read_parquet(${s})`
			editor?.insertAtCursor(s)
		} else if (lang === 'python3') {
			if (!editor?.getCode().includes('import wmill')) {
				editor?.insertAtBeginning('import wmill\n')
			}
			editor?.insertAtCursor(`wmill.load_s3_file(${s})`)
		} else if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
			if (!editor?.getCode().includes('import * as wmill from')) {
				editor?.insertAtBeginning(`import * as wmill from "npm:windmill-client@1"\n`)
			}
			editor?.insertAtCursor(`wmill.loadS3File(${s})`)
		}
	}}
/>

<div class="flex justify-between items-center overflow-y-auto w-full p-0.5">
	<div class="flex items-center">
		<div
			title={validCode ? 'Main function parsable' : 'Main function not parsable'}
			class="rounded-full w-2 h-2 mx-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
		></div>
		<div class="flex items-center gap-0.5">
			{#if showContextVarPicker && customUi?.contextVar != false}
				<Button
					aiId="editor-bar-add-context-variable"
					aiDescription="Add context variable"
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
					aiId="editor-bar-add-variable"
					aiDescription="Add variable"
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

			{#if showS3Picker && customUi?.s3object != false}
				<Button
					aiId="editor-bar-add-s3-object"
					aiDescription="Add S3 Object"
					title="Add S3 object"
					color="light"
					on:click={() => s3FilePicker?.open()}
					size="xs"
					btnClasses="!font-medium text-tertiary"
					spacingSize="md"
					startIcon={{ icon: File }}
					{iconOnly}
					>+S3 Object
				</Button>
			{/if}

			{#if showResourcePicker && customUi?.resource != false}
				<Button
					aiId="editor-bar-add-resource"
					aiDescription="Add resource"
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
					aiId="editor-bar-add-resource-type"
					aiDescription="Add resource type"
					title="Add resource type"
					btnClasses="!font-medium text-tertiary"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={() => resourceTypePicker?.openDrawer()}
					{iconOnly}
					startIcon={{ icon: Package }}
				>
					+Type
				</Button>
			{/if}

			{#if showDatabasePicker && customUi?.database != false}
				<Button
					aiId="editor-bar-add-database"
					aiDescription="Add database"
					title="Add database"
					color="light"
					on:click={() => databasePicker?.openDrawer()}
					size="xs"
					btnClasses="!font-medium text-tertiary"
					spacingSize="md"
					startIcon={{ icon: DatabaseIcon }}
					{iconOnly}
					>+Database
				</Button>
			{/if}

			{#if showDucklakePicker && customUi?.ducklake != false}
				<Button
					aiId="editor-bar-use-ducklake"
					aiDescription="Use Ducklake"
					title="Use Ducklake"
					color="light"
					on:click={() => ducklakePicker?.openDrawer()}
					size="xs"
					btnClasses="!font-medium text-tertiary"
					spacingSize="md"
					startIcon={{ icon: DucklakeIcon }}
					{iconOnly}
					>+Ducklake
				</Button>
			{/if}

			{#if customUi?.reset != false}
				<Button
					aiId="editor-bar-reset-content"
					aiDescription="Reset content"
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
				{#if lang == 'deno' || lang == 'python3' || lang == 'go' || lang == 'bash'}
					<Button
						aiId="editor-bar-reload-assistants"
						aiDescription="Reload assistants"
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
						{#snippet text()}
							Toggle diff mode
						{/snippet}
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
						{#snippet text()}
							Multiplayer
						{/snippet}
						<Users class="ml-1 text-tertiary" size={14} />
					</Popover>
					{#if collabLive}
						<button
							title="Show invite link"
							class="p-1 rounded hover:bg-gray-400 mx-1 border"
							onclick={() => dispatch('collabPopup')}><Link size={14} /></button
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
				<ScriptGen {editor} {diffEditor} {lang} {iconOnly} {args} {openAiChat} />
			{/if}

			<EditorSettings {customUi} />
		</div>
	</div>

	<div class="flex flex-row items-center gap-2">
		{@render right?.()}
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
