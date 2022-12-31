<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import Path from '$lib/components/Path.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faExternalLink, faSave } from '@fortawesome/free-solid-svg-icons'
	import { Eye, Laptop2, Pencil, PenTool, Smartphone } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { sendUserToast } from '../../../utils'
	import type { AppEditorContext, EditorBreakpoint, EditorMode } from '../types'
	import AppExportButton from './AppExportButton.svelte'

	const { app } = getContext<AppEditorContext>('AppEditorContext')
	export let title: string = $app.title || ''
	export let mode: EditorMode
	export let breakpoint: EditorBreakpoint
	const loading = {
		publish: false,
		save: false
	}

	let newPath: string = ''
	let pathError: string | undefined = undefined

	let drawerOpen = false

	function closeDrawer() {
		drawerOpen = false
	}

	async function createApp(path: string) {
		const policy = {
			triggerables: {},
			execution_mode: Policy.execution_mode.PUBLISHER,
			on_behalf_of: `u/${$userStore?.username}`
		}
		try {
			const appId = await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path,
					summary: 'App summary',
					policy
				}
			})
			goto(`/apps/edit/${appId}`)
		} catch (e) {
			sendUserToast('Error creating app', e)
		}
	}

	async function save() {
		if ($page.params.path == undefined) {
			drawerOpen = true
			return
		}
		loading.save = true
		AppService.updateApp({
			workspace: $workspaceStore!,
			path: $page.params.path,
			requestBody: {
				value: $app!,
				summary: title,
				policy: {
					triggerables: {},
					execution_mode: Policy.execution_mode.PUBLISHER,
					on_behalf_of: `u/${$userStore?.username}`
				}
			}
		})
			.then(() => {
				sendUserToast('Saved')
			})
			.catch(() => {
				sendUserToast('Error during saving. Please try again later', true)
			})
			.finally(() => {
				loading.save = false
			})
	}
</script>

<Drawer bind:open={drawerOpen} size="800px">
	<DrawerContent title="Create an App" on:close={() => closeDrawer()}>
		<Path
			bind:error={pathError}
			bind:path={newPath}
			initialPath=""
			namePlaceholder="my_app"
			kind="app"
		/>

		<div slot="actions">
			<Button
				startIcon={{ icon: faSave }}
				disabled={pathError != ''}
				on:click={() => createApp(newPath)}>Create app</Button
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="border-b flex flex-row justify-between py-1 px-4 items-center">
	<input class="text-sm w-64" bind:value={title} />
	<div class="flex gap-8 items-center">
		<div>
			<ToggleButtonGroup bind:selected={mode}>
				<ToggleButton position="left" value="dnd" size="xs">
					<div class="inline-flex gap-1 items-center">
						<Pencil size={14} />
						Editor
					</div>
				</ToggleButton>
				<ToggleButton position="right" value="preview" size="xs">
					<div class="inline-flex gap-1 items-center"> <Eye size={14} /> Preview</div></ToggleButton
				>
			</ToggleButtonGroup>
		</div>
		<div>
			<ToggleButtonGroup bind:selected={breakpoint}>
				<ToggleButton position="left" value="sm" size="xs">
					<Smartphone size={14} />
				</ToggleButton>
				<ToggleButton position="right" value="lg" size="xs"><Laptop2 size={14} /></ToggleButton>
			</ToggleButtonGroup>
		</div>

		<ToggleButtonGroup bind:selected={$app.fullscreen}>
			<ToggleButton position="left" value={false} size="xs">Centered</ToggleButton>
			<ToggleButton position="right" value={true} size="xs">Full Width</ToggleButton>
		</ToggleButtonGroup>
	</div>
	<div class="flex flex-row gap-4 justify-end">
		<AppExportButton app={$app} />

		<Button
			on:click={() => sendUserToast('Publishing apps publically at secret urls is coming soon')}
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faExternalLink }}
		>
			Publish
		</Button>
		<Button
			loading={loading.save}
			startIcon={{ icon: faSave }}
			on:click={save}
			color="dark"
			size="xs">Save</Button
		>
	</div>
</div>
