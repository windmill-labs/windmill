<script module lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1420

	function getImportWmillTsStatement(lang: string | undefined) {
		if (lang === 'deno') {
			return `import * as wmill from "npm:windmill-client@1"\n`
		} else if (lang === 'bun' || lang === 'bunnative') {
			return `import * as wmill from "windmill-client"\n`
		} else if (lang === 'nativets') {
			return `import * as wmill from "./windmill.ts"\n`
		}
		return ''
	}
</script>

<script lang="ts">
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
		GitBranch,
		History,
		Library,
		Link,
		Package,
		Plus,
		RotateCw,
		Save,
		Settings,
		Users
	} from 'lucide-svelte'
	import { capitalize, formatS3Object, toCamel } from '$lib/utils'
	import type { Schema, SchemaProperty, SupportedLanguage } from '$lib/common'
	import ScriptVersionHistory from './ScriptVersionHistory.svelte'
	import { getResetCode } from '$lib/script_helpers'
	import Popover from './Popover.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'
	import type { EditorBarUi } from './custom_ui'
	import EditorSettings from './EditorSettings.svelte'
	import { quicktype, InputData, JSONSchemaInput, FetchingJSONSchemaStore } from 'quicktype-core'
	import S3FilePicker from './S3FilePicker.svelte'
	import DucklakeIcon from './icons/DucklakeIcon.svelte'
	import FlowInlineScriptAiButton from './copilot/FlowInlineScriptAIButton.svelte'
	import GitRepoPopoverPicker from './GitRepoPopoverPicker.svelte'
	import { insertDelegateToGitRepoInCode } from '$lib/ansibleUtils'

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
		args?: Record<string, any>
		noHistory?: boolean
		saveToWorkspace?: boolean
		customUi?: EditorBarUi
		lastDeployedCode?: string | undefined
		diffMode?: boolean
		showHistoryDrawer?: boolean
		right?: import('svelte').Snippet
		openAiChat?: boolean
		moduleId?: string
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
		noHistory = false,
		saveToWorkspace = false,
		customUi = {},
		lastDeployedCode = undefined,
		diffMode = false,
		showHistoryDrawer = $bindable(false),
		right,
		openAiChat = false,
		moduleId = undefined
	}: Props = $props()

	let contextualVariablePicker: ItemPicker | undefined = $state()
	let variablePicker: ItemPicker | undefined = $state()
	let resourcePicker: ItemPicker | undefined = $state()
	let resourceTypePicker: ItemPicker | undefined = $state()
	let variableEditor: VariableEditor | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
	let ducklakePicker: ItemPicker | undefined = $state()
	let dataTablePicker: ItemPicker | undefined = $state()
	let databasePicker: ItemPicker | undefined = $state()
	let gitRepoPickerOpen = $state(false)

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
			'java',
			'ruby',
			'postgresql',
			'mysql',
			'bigquery',
			'mssql',
			'oracledb',
			'snowflake',
			'duckdb'
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
			'java',
			'ruby'
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
			'java',
			'ruby'
			// for related places search: ADD_NEW_LANG
		].includes(lang ?? '')
	)

	let showS3Picker = $derived(
		['duckdb', 'python3'].includes(lang ?? '') ||
			['typescript', 'javascript'].includes(scriptLangToEditorLang(lang))
	)
	let showDucklakePicker = $derived(
		['duckdb', 'python3'].includes(lang ?? '') ||
			['typescript', 'javascript'].includes(scriptLangToEditorLang(lang))
	)
	let showDataTablePicker = $derived(
		['duckdb', 'python3'].includes(lang ?? '') ||
			['typescript', 'javascript'].includes(scriptLangToEditorLang(lang))
	)
	let showDatabasePicker = $derived(['duckdb'].includes(lang ?? ''))
	let showGitRepoPicker = $derived(lang === 'ansible')

	let showResourceTypePicker = $derived(
		['typescript', 'javascript'].includes(scriptLangToEditorLang(lang)) ||
			lang === 'python3' ||
			lang === 'php' ||
			lang === 'rust'
	)

	let codeViewer: Drawer | undefined = $state()
	let codeObj: { language: SupportedLanguage; content: string } | undefined = $state(undefined)

	function insertDelegateToGitRepo(resourcePath: string) {
		if (!editor) return

		const currentCode = editor.getCode()
		const newCode = insertDelegateToGitRepoInCode(currentCode, resourcePath)
		editor.setCode(newCode)
	}

	function addEditorActions() {
		editor?.addAction('insert-variable', 'Windmill: Insert variable', () => {
			variablePicker?.openDrawer()
		})
		editor?.addAction('insert-resource', 'Windmill: Insert resource', () => {
			resourcePicker?.openDrawer()
		})
	}

	$effect(() => {
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
					'leading-comments': false,
					density: 'dense',
					'derive-debug': true
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
		} else if (lang == 'ruby') {
			editor.insertAtCursor(`ENV['${name}']`)
		} else if (
			['postgresql', 'mysql', 'bigquery', 'mssql', 'oracledb', 'snowflake', 'duckdb'].includes(
				lang ?? ''
			)
		) {
			editor.insertAtCursor(`%%${name}%%`)
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
		if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(getImportWmillTsStatement(lang))
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
			editor.insertAtCursor(`wmill variable get ${path} --json | jq -r .value`)
		} else if (lang == 'powershell') {
			editor.insertAtCursor(`$Headers = @{\n"Authorization" = "Bearer $Env:WM_TOKEN"`)
			editor.arrowDown()
			editor.insertAtCursor(
				`\nInvoke-RestMethod -Headers $Headers -Uri "$Env:BASE_INTERNAL_URL/api/w/$Env:WM_WORKSPACE/variables/get_value/${path}"`
			)
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
		} else if (lang == 'ruby') {
			if (!editor.getCode().includes("require 'windmill/mini'")) {
				editor.insertAtBeginning("require 'windmill/mini'\n")
			}
			editor.insertAtCursor(`get_variable("${path}")`)
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
				variant="accent"
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
		if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(getImportWmillTsStatement(lang))
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
			editor.insertAtCursor(`wmill resource get ${path} --json | jq .value`)
		} else if (lang == 'powershell') {
			editor.insertAtCursor(`$Headers = @{\n"Authorization" = "Bearer $Env:WM_TOKEN"`)
			editor.arrowDown()
			editor.insertAtCursor(
				`\nInvoke-RestMethod -Headers $Headers -Uri "$Env:BASE_INTERNAL_URL/api/w/$Env:WM_WORKSPACE/resources/get_value_interpolated/${path}"`
			)
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
		} else if (lang == 'ruby') {
			if (!editor.getCode().includes("require 'windmill/mini'")) {
				editor.insertAtBeginning("require 'windmill/mini'\n")
			}
			editor.insertAtCursor(`get_resource("${path}")`)
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
				variant="accent"
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
			if (lang === 'duckdb') {
				const connStr = name == 'main' ? 'ducklake' : `ducklake://${name}`
				editor?.insertAtCursor(`ATTACH '${connStr}' AS dl; USE dl;\n`)
			} else if (lang === 'python3') {
				if (!editor?.getCode().includes('import wmill')) {
					editor?.insertAtBeginning('import wmill\n')
				}
				editor?.insertAtCursor(`dl = wmill.ducklake(${name == 'main' ? '' : `'${name}'`})\n`)
			} else if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
				if (!editor?.getCode().includes('import * as wmill from')) {
					editor?.insertAtBeginning(getImportWmillTsStatement(lang))
				}
				editor?.insertAtCursor(`let sql = wmill.ducklake(${name == 'main' ? '' : `'${name}'`})\n`)
			}
		}}
		tooltip="Attach a Ducklake to your scripts. Ducklake allows you to manipulate large data on S3 blob files through a traditional SQL interface."
		documentationLink="https://www.windmill.dev/docs/core_concepts/persistent_storage/ducklake"
		itemName="ducklake"
		loadItems={async () =>
			(await WorkspaceService.listDucklakes({ workspace: $workspaceStore ?? 'NO_W' })).map(
				(path) => ({ path })
			)}
	>
		{#snippet submission()}
			<div class="flex flex-row gap-x-1 mr-2">
				<Button
					startIcon={{ icon: Settings }}
					target="_blank"
					variant="accent"
					href="{base}/workspace_settings?tab=windmill_lfs"
				>
					Go to settings
				</Button>
			</div>
		{/snippet}
	</ItemPicker>
{/if}

{#if showDataTablePicker}
	<ItemPicker
		bind:this={dataTablePicker}
		pickCallback={async (_, name) => {
			if (lang === 'duckdb') {
				const connStr = name == 'main' ? 'datatable' : `datatable://${name}`
				editor?.insertAtCursor(`ATTACH '${connStr}' AS dt; USE dt;\n`)
			} else if (lang === 'python3') {
				if (!editor?.getCode().includes('import wmill')) {
					editor?.insertAtBeginning('import wmill\n')
				}
				editor?.insertAtCursor(`db = wmill.datatable(${name == 'main' ? '' : `'${name}'`})\n`)
			} else if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
				if (!editor?.getCode().includes('import * as wmill from')) {
					editor?.insertAtBeginning(getImportWmillTsStatement(lang))
				}
				editor?.insertAtCursor(`let sql = wmill.datatable(${name == 'main' ? '' : `'${name}'`})\n`)
				editor?.insertAtCursor(`let query_result = await sql\`SELECT * FROM _\`.fetchOne()\n`)
			}
		}}
		tooltip="Attach a datatable to your script."
		documentationLink="https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables"
		itemName="data table"
		loadItems={async () =>
			(await WorkspaceService.listDataTables({ workspace: $workspaceStore ?? 'NO_W' })).map(
				(path) => ({ path })
			)}
	>
		{#snippet submission()}
			<div class="flex flex-row gap-x-1 mr-2">
				<Button
					startIcon={{ icon: Settings }}
					target="_blank"
					variant="accent"
					href="{base}/workspace_settings?tab=windmill_data_tables"
				>
					Go to settings
				</Button>
			</div>
		{/snippet}
	</ItemPicker>
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
	onSelectAndClose={(s3obj) => {
		let s = `'${formatS3Object(s3obj)}'`
		if (lang === 'duckdb') {
			editor?.insertAtCursor(`SELECT * FROM ${s};`)
		} else if (lang === 'python3') {
			if (!editor?.getCode().includes('import wmill')) {
				editor?.insertAtBeginning('import wmill\n')
			}
			editor?.insertAtCursor(`wmill.load_s3_file(${s})`)
		} else if (['javascript', 'typescript'].includes(scriptLangToEditorLang(lang))) {
			if (!editor?.getCode().includes('import * as wmill from')) {
				editor?.insertAtBeginning(getImportWmillTsStatement(lang))
			}
			editor?.insertAtCursor(`wmill.loadS3File(${s})`)
		}
	}}
/>

<div class="flex justify-between items-center overflow-y-auto w-full p-0.5">
	<div class="flex gap-3 items-center">
		<div
			title={validCode ? 'Main function parsable' : 'Main function not parsable'}
			class="rounded-full w-2 h-2 mx-2 {validCode ? 'bg-green-300' : 'bg-red-300'}"
		></div>
		<div class="flex items-center gap-2">
			{#if showContextVarPicker && customUi?.contextVar != false}
				<Button
					aiId="editor-bar-add-context-variable"
					aiDescription="Add context variable"
					title="Add context variable"
					variant="subtle"
					on:click={contextualVariablePicker.openDrawer}
					unifiedSize="sm"
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
					variant="subtle"
					on:click={variablePicker.openDrawer}
					unifiedSize="sm"
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
					variant="subtle"
					on:click={() => s3FilePicker?.open()}
					unifiedSize="sm"
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
					unifiedSize="sm"
					variant="subtle"
					on:click={resourcePicker.openDrawer}
					{iconOnly}
					startIcon={{ icon: Package }}
				>
					+Resource
				</Button>
			{/if}

			{#if showGitRepoPicker && customUi?.resource != false}
				<GitRepoPopoverPicker
					bind:isOpen={gitRepoPickerOpen}
					on:selected={(e) => insertDelegateToGitRepo(e.detail.resourcePath)}
				>
					<Button
						aiId="editor-bar-add-git-repo"
						aiDescription="Delegate to Git repository"
						title="Delegate to Git repository"
						unifiedSize="sm"
						variant="subtle"
						on:click={() => (gitRepoPickerOpen = true)}
						{iconOnly}
						startIcon={{ icon: GitBranch }}
					>
						+Git Repo
					</Button>
				</GitRepoPopoverPicker>
			{/if}

			{#if showResourceTypePicker && customUi?.type != false}
				<Button
					aiId="editor-bar-add-resource-type"
					aiDescription="Add resource type"
					title="Add resource type"
					variant="subtle"
					unifiedSize="sm"
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
					variant="subtle"
					on:click={() => databasePicker?.openDrawer()}
					unifiedSize="sm"
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
					variant="subtle"
					on:click={() => ducklakePicker?.openDrawer()}
					unifiedSize="sm"
					startIcon={{ icon: DucklakeIcon }}
					{iconOnly}
					>+Ducklake
				</Button>
			{/if}

			{#if showDataTablePicker && customUi?.dataTable != false}
				<Button
					aiId="editor-bar-use-datatable"
					aiDescription="Use DataTable"
					title="Use DataTable"
					variant="subtle"
					on:click={() => dataTablePicker?.openDrawer()}
					unifiedSize="sm"
					startIcon={{ icon: DatabaseIcon }}
					{iconOnly}
					>+Data table
				</Button>
			{/if}

			{#if customUi?.reset != false}
				<Button
					aiId="editor-bar-reset-content"
					aiDescription="Reset content"
					title="Reset Content"
					unifiedSize="sm"
					variant="subtle"
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
						unifiedSize="sm"
						variant="subtle"
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
						options={{ right: 'Diff' }}
						size="sm"
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
						<DiffIcon class="ml-1 text-primary" size={14} />
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
						<Users class="ml-1 text-primary" size={14} />
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
									<span class="text-sm font-semibold leading-none text-white">
										{user.name.substring(0, 2).toLocaleUpperCase()}
									</span>
								</span>
							{/each}
						</div>
					{/if}
				</div>
			{/if}

			{#if customUi?.aiGen != false}
				{#if openAiChat}
					<FlowInlineScriptAiButton {moduleId} btnProps={{ variant: 'subtle' }} />
				{/if}
			{/if}

			<EditorSettings {customUi} btnProps={{ variant: 'subtle', unifiedSize: 'md' }} />
		</div>
	</div>

	<div class="flex flex-row items-center gap-2 whitespace-nowrap">
		{@render right?.()}
		{#if scriptPath && !noHistory}
			<Button
				unifiedSize="sm"
				variant="subtle"
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
				unifiedSize="sm"
				variant="subtle"
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
				unifiedSize="sm"
				variant="subtle"
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
