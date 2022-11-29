<script lang="ts">
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		faDesktopAlt,
		faDisplay,
		faExternalLink,
		faHand,
		faMobileAlt,
		faTabletAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { sendUserToast } from '../../../utils'
	import type { AppEditorContext, EditorBreakpoint, EditorMode } from '../types'

	const { app } = getContext<AppEditorContext>('AppEditorContext')
	export let title: string = $app.title || ''
	export let mode: EditorMode
	export let breakpoint: EditorBreakpoint
	const loading = {
		publish: false,
		save: false
	}

	async function save() {
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
				sendUserToast('Saved successfully.')
			})
			.catch(() => {
				sendUserToast('Error during saving. Please try again later.', true)
			})
			.finally(() => {
				loading.save = false
			})
	}
</script>

<div class="border-b flex flex-row justify-between py-1 px-4 items-center">
	<input class="text-sm w-64" bind:value={title} />
	<div>
		<ToggleButtonGroup bind:selected={mode}>
			<ToggleButton position="left" value="dnd" startIcon={{ icon: faHand }} size="xs">
				Editor
			</ToggleButton>
			<ToggleButton position="right" value="preview" startIcon={{ icon: faDisplay }} size="xs">
				Preview
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
	<div>
		<ToggleButtonGroup bind:selected={breakpoint}>
			<ToggleButton position="left" value="sm" startIcon={{ icon: faMobileAlt }} size="xs">
				Mobile
			</ToggleButton>
			<ToggleButton position="center" value="md" startIcon={{ icon: faTabletAlt }} size="xs">
				Tablet
			</ToggleButton>
			<ToggleButton position="right" value="lg" startIcon={{ icon: faDesktopAlt }} size="xs">
				Desktop
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
	<div class="flex flex-row gap-2">
		<Button color="dark" size="xs" variant="border" startIcon={{ icon: faExternalLink }}>
			Publish
		</Button>
		<Button loading={loading.save} on:click={save} color="dark" size="xs">Save</Button>
	</div>
</div>
