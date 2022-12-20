<script lang="ts">
	import { faMousePointer, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import { Building, Globe2 } from 'lucide-svelte'
	import InlineScriptList from './InlineScriptList.svelte'
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import WorkspaceScriptList from './WorkspaceScriptList.svelte'
	import WorkspaceFlowList from './WorkspaceFlowList.svelte'
	import type { AppEditorContext, GridItem, InlineScript } from '$lib/components/apps/types'
	import { getContext } from 'svelte'

	type Tab = 'hubscripts' | 'workspacescripts' | 'workspaceflows' | 'inlinescripts'

	export let appInput: ResultAppInput

	let tab: Tab = 'inlinescripts'
	let filter: string = ''
	let picker: Drawer

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function pickScript(path: string) {
		if (appInput.type === 'runnable') {
			appInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'script'
			}
		}
	}

	function pickFlow(path: string) {
		if (appInput.type === 'runnable') {
			appInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'flow'
			}
		}
	}

	function pickInlineScript(name: string, inlineScript: InlineScript) {
		if (appInput.type === 'runnable') {
			appInput.runnable = {
				type: 'runnableByName',
				name,
				inlineScript
			}
		}
	}

	function createScript(): string {
		let index = 0
		let newScriptPath = `inline_script_${index}`

		const names = $app.grid.reduce((acc, gridItem: GridItem) => {
			const { componentInput } = gridItem.data

			if (
				componentInput.type === 'runnable' &&
				componentInput.runnable?.type === 'runnableByName'
			) {
				acc.push(componentInput.runnable.name)
			}

			return acc
		}, [] as string[])

		const unusedNames = Object.keys($app.unusedInlineScripts ?? {})

		// Find a name that is not used by any other inline script
		while (names.includes(newScriptPath) || unusedNames.includes(newScriptPath)) {
			newScriptPath = `inline_script_${++index}`
		}

		appInput.runnable = {
			type: 'runnableByName',
			name: newScriptPath,
			inlineScript: undefined
		}

		appInput = appInput

		return newScriptPath
	}
</script>

<Drawer bind:this={picker} size="1000px">
	<DrawerContent title="Picker" on:close={picker.closeDrawer}>
		<div>
			<div class="max-w-6xl">
				<Tabs bind:selected={tab}>
					<Tab size="sm" value="inlinescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Inline Scripts
						</div>
					</Tab>
					<Tab size="sm" value="workspacescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Scripts
						</div>
					</Tab>
					<Tab size="sm" value="workspaceflows">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Flows
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
							<InlineScriptList on:pick={(e) => pickInlineScript('', e.detail)} />
						{:else if tab == 'workspacescripts'}
							<WorkspaceScriptList on:pick={(e) => pickScript(e.detail)} />
						{:else if tab == 'workspaceflows'}
							<WorkspaceFlowList on:pick={(e) => pickFlow(e.detail)} />
						{:else if tab == 'hubscripts'}
							<PickHubScript bind:filter on:pick={(e) => pickScript(e.detail.path)} />
						{/if}
					</div>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>

<div class="flex flex-col gap-2">
	<Button
		on:click={createScript}
		size="sm"
		color="light"
		variant="border"
		startIcon={{ icon: faPlus }}
	>
		Create an inline script
	</Button>
	<Button
		on:click={() => picker?.openDrawer()}
		size="sm"
		color="blue"
		startIcon={{ icon: faMousePointer }}
	>
		Select a script
	</Button>
</div>
