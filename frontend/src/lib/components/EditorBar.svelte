<script context="module" lang="ts">
	export const EDITOR_BAR_WIDTH_THRESHOLD = 1044
</script>

<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { getModifierKey, getScriptByPath, sendUserToast } from '$lib/utils'

	import {
		faBroom,
		faCube,
		faDollarSign,
		faEye,
		faPlus,
		faRotate,
		faRotateLeft
	} from '@fortawesome/free-solid-svg-icons'

	import { workspaceStore } from '$lib/stores'
	import type Editor from './Editor.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import ResourceEditor from './ResourceEditor.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import Button from './common/button/Button.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Badge, Drawer } from './common'
	import WorkspaceScriptPicker from './flows/pickers/WorkspaceScriptPicker.svelte'
	import PickHubScript from './flows/pickers/PickHubScript.svelte'
	import ToggleHubWorkspace from './ToggleHubWorkspace.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Popover from './Popover.svelte'
	import Kbd from './common/kbd/Kbd.svelte'
	import { SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS } from '$lib/consts'

	export let lang: 'python3' | 'deno' | 'go' | 'bash'
	export let editor: Editor | undefined
	export let websocketAlive: { pyright: boolean; black: boolean; deno: boolean; go: boolean }
	export let iconOnly: boolean = false
	export let validCode: boolean = true
	export let kind: 'script' | 'trigger' | 'approval' = 'script'

	let contextualVariablePicker: ItemPicker
	let variablePicker: ItemPicker
	let resourcePicker: ItemPicker
	let variableEditor: VariableEditor
	let resourceEditor: ResourceEditor

	let codeViewer: Drawer
	let codeObj: { language: 'python3' | 'deno' | 'go' | 'bash'; content: string } | undefined =
		undefined

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

	let version = __pkg__.version
</script>

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
		if (!editor) return
		if (lang == 'deno') {
			if (!editor.getCode().includes('import * as wmill from')) {
				editor.insertAtBeginning(
					`import * as wmill from 'https://deno.land/x/windmill@v${version}/mod.ts'\n`
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
  "$BASE_INTERNAL_URL/api/w/$WM_WORKSPACE/variables/get/${path}" \\
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
			startIcon={{ icon: faPlus }}
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
				editor.insertAtBeginning(
					`import * as wmill from 'https://deno.land/x/windmill@v${version}/mod.ts'\n`
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
  "$BASE_INTERNAL_URL/api/w/$WM_WORKSPACE/resources/get/${path}" \\
  | jq -r .value`)
		}
		sendUserToast(`${path} inserted at cursor`)
	}}
	itemName="Resource"
	buttons={{ 'Edit/View': (x) => resourceEditor.initEdit(x) }}
	extraField="description"
	extraField2="resource_type"
	loadItems={async () =>
		await ResourceService.listResource({ workspace: $workspaceStore ?? 'NO_W' })}
>
	<div slot="submission" class="flex flex-row gap-x-1 mr-2">
		<Button
			target="_blank"
			variant="border"
			color="blue"
			size="sm"
			href="/resources?connect_app=undefined"
		>
			Connect an API
		</Button>
	</div>
</ItemPicker>

<ResourceEditor bind:this={resourceEditor} on:refresh={resourcePicker.openDrawer} />
<VariableEditor bind:this={variableEditor} on:create={variablePicker.openDrawer} />

<div class="flex justify-between items-center overflow-y-auto w-full p-1">
	<div class="flex items-center">
		<Badge color={validCode ? 'green' : 'red'} class="min-w-[60px] mr-3">
			{validCode ? 'Valid' : 'Invalid'}
		</Badge>
		<div class="flex items-center divide-x">
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
				class="pr-1"
				disablePopup={!iconOnly}
			>
				<Button
					color="light"
					btnClasses="!font-medium !h-full"
					on:click={contextualVariablePicker.openDrawer}
					size="xs"
					spacingSize="md"
					startIcon={{ icon: faDollarSign }}
					{iconOnly}
				>
					+Context Var
				</Button>
				<svelte:fragment slot="text">Add context variable</svelte:fragment>
			</Popover>
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
				class="px-1"
				disablePopup={!iconOnly}
			>
				<Button
					color="light"
					btnClasses="!font-medium !h-full"
					on:click={variablePicker.openDrawer}
					size="xs"
					spacingSize="md"
					startIcon={{ icon: faDollarSign }}
					{iconOnly}
				>
					+Variable
				</Button>
				<svelte:fragment slot="text">Add variable</svelte:fragment>
			</Popover>
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
				class="px-1"
				disablePopup={!iconOnly}
			>
				<Button
					btnClasses="!font-medium !h-full"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={resourcePicker.openDrawer}
					{iconOnly}
					startIcon={{ icon: faCube }}
				>
					+Resource
				</Button>
				<svelte:fragment slot="text">Add resource</svelte:fragment>
			</Popover>
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
				class="px-1"
				disablePopup={!iconOnly}
			>
				<Button
					btnClasses="!font-medium !h-full"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={editor?.clearContent}
					{iconOnly}
					startIcon={{ icon: faRotateLeft }}
				>
					Reset
				</Button>
				<svelte:fragment slot="text">Reset</svelte:fragment>
			</Popover>
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
				class="px-1"
				disablePopup={!iconOnly}
			>
				<Button
					btnClasses="!font-medium !h-full"
					size="xs"
					spacingSize="md"
					color="light"
					on:click={editor?.reloadWebsocket}
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
				<svelte:fragment slot="text">Reload assistant</svelte:fragment>
			</Popover>
			<Popover
				notClickable
				placement="bottom"
				disapperTimoout={0}
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
			</Popover>
		</div>
	</div>
	<Popover
		notClickable
		placement="bottom"
		disapperTimoout={0}
		class="px-1"
		disablePopup={!iconOnly}
	>
		{#if SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS}
		<Button
			btnClasses="!font-medium"
			size="xs"
			spacingSize="md"
			color="light"
			on:click={scriptPicker.openDrawer}
			{iconOnly}
			startIcon={{ icon: faEye }}
		>
			Explore other scripts
		</Button>
		{/if}
		<svelte:fragment slot="text">Script</svelte:fragment>
	</Popover>
</div>

<style lang="postcss">
	span.green {
		@apply text-green-600 animate-[pulse_5s_ease-in-out_infinite];
	}
</style>
