<script lang="ts">
	import { workspaceStore, usersWorkspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	let colorEnabled = false
	let workspaceColor = $usersWorkspaceStore?.workspaces.find(w => w.id === $workspaceStore)?.color

	export let open = false

	$: if (colorEnabled && !workspaceColor) generateRandomColor()

	function generateRandomColor() {
		const randomColor =
			'#' +
			Math.floor(Math.random() * 16777215)
				.toString(16)
				.padStart(6, '0')
		workspaceColor = randomColor
	}

	async function changeWorkspaceColor() {
		const colorToSave = colorEnabled && workspaceColor ? workspaceColor : undefined
		open = false
		await WorkspaceService.changeWorkspaceColor({
			workspace: $workspaceStore!,
			requestBody: {
				color: colorToSave
			}
		})

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		workspaceColor = colorToSave

		sendUserToast(`Workspace color updated.`)
	}
</script>

<div>
	<p class="font-semibold text-sm">Workspace Color</p>
	<div class="flex flex-row gap-0.5 items-center">
		{#if workspaceColor}
			<div
				class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-600"
				style="background-color: {workspaceColor}"
			/>
		{:else}
			<span class="text-xs text-secondary">No color set</span>
		{/if}
		<Button
			on:click={() => {
				open = true
			}}
			size="xs"
			spacingSize="xs2"
			color="light"
			iconOnly
			startIcon={{
				icon: Pen
			}}
		/>
	</div>
</div>

<Modal bind:open title="Change Workspace Color">
	<div class="flex flex-col gap-4">
		<label class="block">
			<span class="text-secondary text-sm">Workspace color</span>
			<div class="flex items-center gap-2">
				<Toggle bind:checked={colorEnabled} options={{ right: 'Enable' }} />
				<input class="w-10" type="color" bind:value={workspaceColor} disabled={!colorEnabled} />
				<input
					type="text"
					class="w-24 text-sm"
					bind:value={workspaceColor}
					disabled={!colorEnabled}
				/>
				<Button on:click={generateRandomColor} size="xs" disabled={!colorEnabled}>Random</Button>
			</div>
		</label>
	</div>

	<svelte:fragment slot="actions">
		<Button
			size="sm"
			on:click={() => {
				changeWorkspaceColor()
			}}
		>
			Save
		</Button>
	</svelte:fragment>
</Modal>
