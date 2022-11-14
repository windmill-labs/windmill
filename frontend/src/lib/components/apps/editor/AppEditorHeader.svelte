<script lang="ts">
	import { page } from '$app/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faArrowsLeftRight, faExternalLink, faHand } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, EditorMode } from '../types'

	export let title: string
	export let mode: EditorMode

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	async function save() {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $page.params.path,
			requestBody: {
				value: $app!,
				summary: 'App summary',
				policy: {
					triggerables: {},
					execution_mode: Policy.execution_mode.PUBLISHER,
					on_behalf_of: `u/${$userStore?.username}`
				}
			}
		})
		console.log('App saved')
	}
</script>

<div class="border-b h-12 flex flex-row justify-between py-2 px-4 items-center">
	<span class="text-sm">{title}</span>
	<div>
		<ToggleButtonGroup bind:selected={mode}>
			<ToggleButton position="left" value="dnd" startIcon={{ icon: faHand }} size="sm">
				Component editor
			</ToggleButton>
			<ToggleButton
				position="right"
				value="width"
				startIcon={{ icon: faArrowsLeftRight }}
				size="sm"
			>
				Width editor
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
	<div class="flex flex-row gap-2">
		<Button color="dark" size="sm" variant="border" startIcon={{ icon: faExternalLink }}>
			Publish
		</Button>
		<Button on:click={save} size="sm">Save</Button>
	</div>
</div>
