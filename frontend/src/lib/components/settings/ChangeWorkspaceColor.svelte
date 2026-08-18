<script lang="ts">
	import { workspaceStore, usersWorkspaceStore, userWorkspaces, workspaceColor } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkspaceScopeTrigger from '../WorkspaceScopeTrigger.svelte'
	import DarkModeToggle from '../sidebar/DarkModeToggle.svelte'

	let { open = false }: { open?: boolean } = $props()

	// A fork's color themes the fork pickers, so the section takes the
	// create-fork form's "Fork color" shape — live picker preview included.
	const isFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces))
	const colorLabel = $derived(isFork ? 'Fork color' : 'Workspace color')

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

<div class="flex flex-col gap-1">
	<p class="font-semibold text-xs text-emphasis">{colorLabel}</p>
	<p class="text-xs text-secondary font-normal">
		{isFork
			? 'Color to identify this fork in the pickers'
			: 'Color to identify the current workspace in the list of workspaces'}
	</p>
	<div class="flex flex-row gap-0.5 items-center">
		{#if $workspaceColor}
			<div
				class="w-10 h-6 rounded-md border border-gray-300 dark:border-gray-600"
				style="background-color: {$workspaceColor}"
			></div>
		{:else}
			<span class="text-xs font-normal text-primary">No color set</span>
		{/if}
		<Button
			on:click={() => {
				open = true
			}}
			unifiedSize="sm"
			variant="subtle"
			iconOnly
			startIcon={{
				icon: Pen
			}}
		/>
	</div>
</div>

<Modal bind:open title="Change {colorLabel.toLowerCase()}">
	<div class="flex flex-col gap-4">
		<label class="block">
			<span class="text-secondary text-sm">{colorLabel}</span>
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
		{#if isFork && colorEnabled}
			<div class="flex flex-col gap-1">
				<div class="flex flex-row items-center gap-1">
					<span class="text-2xs text-secondary">Fork picker preview</span>
					<DarkModeToggle forcedDarkMode={false} />
				</div>
				<div class="w-64">
					<WorkspaceScopeTrigger
						workspaceId={$workspaceStore}
						color={editingColor}
						showChevron={false}
						wrap
						class="w-full pointer-events-none"
					/>
				</div>
			</div>
		{/if}
	</div>

	{#snippet actions()}
		<Button
			size="sm"
			variant="accent"
			on:click={() => {
				changeWorkspaceColor()
			}}
		>
			Save
		</Button>
	{/snippet}
</Modal>
