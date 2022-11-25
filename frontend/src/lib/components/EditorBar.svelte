<script context="module" lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1044
</script>

<script lang="ts">
	import { ResourceService, ScriptService, VariableService } from '$lib/gen'
	import { getScriptByPath, loadHubScripts, sendUserToast } from '$lib/utils'

	import {
		faCode,
		faCube,
		faDollarSign,
		faEye,
		faRotate,
		faRotateLeft,
		faWallet
	} from '@fortawesome/free-solid-svg-icons'

	import { hubScripts, workspaceStore } from '$lib/stores'
	import type Editor from './Editor.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import ResourceEditor from './ResourceEditor.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import Button from './common/button/Button.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Badge, Drawer } from './common'

	export let lang: 'python3' | 'deno' | 'go' | 'bash'
	export let editor: Editor
	export let websocketAlive: { pyright: boolean; black: boolean; deno: boolean; go: boolean }
	export let iconOnly: boolean = false
	export let validCode: boolean = true

	let contextualVariablePicker: ItemPicker
	let variablePicker: ItemPicker
	let resourcePicker: ItemPicker
	let scriptPicker: ItemPicker
	let variableEditor: VariableEditor
	let resourceEditor: ResourceEditor

	let codeViewer: Drawer
	let codeLang: 'python3' | 'deno' | 'go' | 'bash' = 'deno'
	let codeContent: string = ''

	function addEditorActions() {
		editor.addAction('insert-variable', 'Windmill: Insert variable', () => {
			variablePicker.openDrawer()
		})
		editor.addAction('insert-resource', 'Windmill: Insert resource', () => {
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

	async function loadScripts(): Promise<{ path: string; summary?: string }[]> {
		const workspaceScripts: { path: string; summary?: string }[] = await ScriptService.listScripts({
			workspace: $workspaceStore ?? 'NO_W'
		})
		await loadHubScripts()
		const hubScripts_ = $hubScripts ?? []

		return workspaceScripts.concat(hubScripts_)
	}
</script>

<ItemPicker
	bind:this={scriptPicker}
	pickCallback={async (path, _) => {
		const { language, content } = await getScriptByPath(path ?? '')
		codeContent = content
		codeLang = language
		codeViewer.openDrawer()
	}}
	closeOnClick={false}
	itemName="script"
	extraField="summary"
	loadItems={loadScripts}
/>

<Drawer bind:this={codeViewer} size="600px">
	<DrawerContent title="Code" on:close={codeViewer.closeDrawer}>
		<HighlightCode language={codeLang} code={codeContent} />
	</DrawerContent>
</Drawer>

<ItemPicker
	bind:this={contextualVariablePicker}
	pickCallback={(path, name) => {
		if (lang == 'deno') {
			editor.insertAtCursor(`Deno.env.get('${name}')`)
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
	itemName="Contextual Variable"
	extraField="name"
	loadItems={loadContextualVariables}
/>

<ItemPicker
	bind:this={variablePicker}
	pickCallback={(path, name) => {
		if (lang == 'deno') {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(
					`import * as wmill from 'https://deno.land/x/windmill@v${__pkg__.version}/mod.ts'\n`
				)
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
  "$WM_BASE_URL/api/w/$WM_WORKSPACE/variables/get/${path}" \\
  | jq -r .value`)
		}
		sendUserToast(`${name} inserted at cursor`)
	}}
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
			on:click={() => {
				variableEditor.initNew()
			}}
		>
			Create a new variable
		</Button>
	</div>
</ItemPicker>

<ItemPicker
	bind:this={resourcePicker}
	pickCallback={(path, _) => {
		if (lang == 'deno') {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(
					`import * as wmill from 'https://deno.land/x/windmill@v${__pkg__.version}/mod.ts'\n`
				)
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
  "$WM_BASE_URL/api/w/$WM_WORKSPACE/resources/get/${path}" \\
  | jq -r .value`)
		}
		sendUserToast(`${path} inserted at cursor`)
	}}
	itemName="Resource"
	buttons={{ 'Edit/View': (x) => resourceEditor.initEdit(x) }}
	extraField="description"
	loadItems={async () =>
		await ResourceService.listResource({ workspace: $workspaceStore ?? 'NO_W' })}
>
	<div slot="submission" class="flex flex-row gap-x-1">
		<Button target="_blank" color="blue" size="sm" href="/resources?connect_app=undefined">
			Connect an API
		</Button>
		<Button
			variant="border"
			color="blue"
			size="sm"
			on:click={() => {
				resourceEditor.initNew()
			}}
		>
			New custom resource
		</Button>
	</div>
</ItemPicker>

<ResourceEditor bind:this={resourceEditor} on:refresh={resourcePicker.openDrawer} />
<VariableEditor bind:this={variableEditor} on:create={variablePicker.openDrawer} />

<div class="flex flex-row justify-between items-center overflow-hidden w-full px-1">
	<div class="flex flex-row divide-x items-center">
		<div class="mx-2">
			{#if validCode}
				<Badge color="green">Inputs</Badge>
			{:else}
				<Badge color="red">Inputs</Badge>
			{/if}
		</div>
		<div>
			<Button
				color="light"
				btnClasses="mr-1 !font-medium"
				on:click={contextualVariablePicker.openDrawer}
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faDollarSign }}
				{iconOnly}
			>
				+Context Var
			</Button>
		</div>
		<div>
			<Button
				color="light"
				btnClasses="mx-1 !font-medium"
				on:click={variablePicker.openDrawer}
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faWallet }}
				{iconOnly}
			>
				+Var
			</Button>
		</div>
		<div>
			<Button
				btnClasses="mx-1 !font-medium"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={resourcePicker.openDrawer}
				{iconOnly}
				startIcon={{ icon: faCube }}
			>
				+Resource
			</Button>
		</div>

		<div>
			<Button
				btnClasses="mx-1 !font-medium"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={scriptPicker.openDrawer}
				{iconOnly}
				startIcon={{ icon: faEye }}
			>
				Script
			</Button>
		</div>

		<div>
			<Button
				btnClasses="mx-1 !font-medium"
				size="xs"
				spacingSize="md"
				color="light"
				on:click={editor.clearContent}
				{iconOnly}
				startIcon={{ icon: faRotateLeft }}
			>
				Reset
			</Button>
		</div>
	</div>
	<div class="py-1">
		<Button
			btnClasses="!font-medium"
			size="xs"
			spacingSize="md"
			color="light"
			on:click={editor.reloadWebsocket}
			startIcon={{ icon: faRotate }}
		>
			{#if !iconOnly}
				Assistant
			{/if}
			<span class="ml-1 -my-1">
				{#if lang == 'deno'}
					(<span class={websocketAlive.deno ? 'green' : 'text-red-700'}>Deno</span>)
				{:else if lang == 'go'}
					(<span class={websocketAlive.go ? 'green' : 'text-red-700'}>Go</span>)
				{:else if lang == 'python3'}
					(<span class={websocketAlive.pyright ? 'green' : 'text-red-700'}>Pyright</span>
					<span class={websocketAlive.black ? 'green' : 'text-red-700'}>Black</span>)
				{/if}
			</span>
		</Button>
	</div>
</div>

<style>
	span.green {
		@apply text-green-600 animate-[pulse_5s_ease-in-out_infinite];
	}
</style>
