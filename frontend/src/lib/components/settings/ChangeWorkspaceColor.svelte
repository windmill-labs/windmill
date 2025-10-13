<script lang="ts">
	import { workspaceStore, usersWorkspaceStore, workspaceColor } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	let { open = false }: { open?: boolean } = $props()

	let colorEnabled = $state(false)
	let editingColor = $state<string | undefined>(undefined)
	let lastWorkspace = $state<string | undefined>(undefined)

	$effect(() => {
		if ($workspaceStore !== lastWorkspace) {
			lastWorkspace = $workspaceStore
			editingColor = $workspaceColor ?? undefined
			colorEnabled = !!$workspaceColor
		}
	})

	$effect(() => {
		if (colorEnabled && !editingColor) {
			generateRandomColor()
		}
	})

	function generateRandomColor() {
		const randomColor =
			'#' +
			Math.floor(Math.random() * 16777215)
				.toString(16)
				.padStart(6, '0')
		editingColor = randomColor
	}

	async function changeWorkspaceColor() {
		const colorToSave = colorEnabled && editingColor ? editingColor : undefined
		open = false
		await WorkspaceService.changeWorkspaceColor({
			workspace: $workspaceStore!,
			requestBody: {
				color: colorToSave
			}
		})

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		sendUserToast(`Workspace color updated.`)
	}
</script>

<div>
	<p class="font-semibold text-sm">Workspace color</p>
	<div class="flex flex-row gap-0.5 items-center">
		{#if $workspaceColor}
			<div
				class="w-5 h-5 rounded-full border border-gray-300 dark:border-gray-600"
				style="background-color: {$workspaceColor}"
			></div>
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
	<p class="italic text-xs"> Color to identify the current workspace in the list of workspaces </p>
</div>

<Modal bind:open title="Change Workspace Color">
	<div class="flex flex-col gap-4">
		<label class="block">
			<span class="text-secondary text-sm">Workspace color</span>
			<div class="flex items-center gap-2">
				<Toggle bind:checked={colorEnabled} options={{ right: 'Enable' }} />
				{#if colorEnabled}
					<input class="w-10" type="color" bind:value={editingColor} disabled={!colorEnabled} />
				{/if}
				<input
					type="text"
					class="w-24 text-sm"
					bind:value={editingColor}
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
