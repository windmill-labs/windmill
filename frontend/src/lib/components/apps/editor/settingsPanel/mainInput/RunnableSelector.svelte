<script lang="ts">
	import { faMousePointer } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import { Building, Globe2 } from 'lucide-svelte'
	import InlineScriptList from './InlineScriptList.svelte'
	import type { AppInput } from '$lib/components/apps/inputType'
	import WorkspaceScriptList from './WorkspaceScriptList.svelte'
	import WorkspaceFlowList from './WorkspaceFlowList.svelte'

	type Tab = 'hubscripts' | 'hubflows' | 'workspacescripts' | 'workspaceflows' | 'inlinescripts'

	export let inlineScripts: string[]

	export let componentInput: AppInput
	let tab: Tab = 'workspacescripts'
	let filter: string = ''

	let picker: Drawer

	function pickScript(path: string) {
		if (componentInput.type === 'runnable') {
			componentInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'script'
			}
		}
	}

	function pickFlow(path: string) {
		if (componentInput.type === 'runnable') {
			componentInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'flow'
			}
		}
	}

	function pickInlineScript(inlineScriptName: string) {
		if (componentInput.type === 'runnable') {
			componentInput.runnable = {
				type: 'runnableByName',
				inlineScriptName
			}
		}
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
							<InlineScriptList {inlineScripts} on:pick={(e) => pickInlineScript(e.detail)} />
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

<Button
	on:click={() => picker?.openDrawer()}
	size="sm"
	spacingSize="md"
	color="blue"
	startIcon={{ icon: faMousePointer }}
>
	Pick
</Button>
