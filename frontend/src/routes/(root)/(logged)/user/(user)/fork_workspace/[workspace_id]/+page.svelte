<script lang="ts">
	import { run } from 'svelte/legacy'

	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import {
		WorkspaceService,
		SettingService,
		UserService
	} from '$lib/gen'
	import { validateUsername } from '$lib/utils'
	import { logoutWithRedirect } from '$lib/logout'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, userStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'

	const rd = $page.url.searchParams.get('rd')
	const parentWorkspaceId = $page.params.workspace_id

	let id = $state('')
	let name = $state('')
	let username = $state('')

	let errorId = $state('')
	let errorUser = $state('')
	let checking = $state(false)

	let workspaceColor: string | null = $state(null)
	let colorEnabled = $state(false)

	// Load parent workspace info
	let parentWorkspace = $state<{ id: string; name: string } | null>(null)

	function generateRandomColor() {
		const randomColor =
			'#' +
			Math.floor(Math.random() * 16777215)
				.toString(16)
				.padStart(6, '0')
		workspaceColor = randomColor
	}

	async function validateName(id: string): Promise<void> {
		checking = true
		let exists = await WorkspaceService.existsWorkspace({ requestBody: { id } })
		if (exists) {
			errorId = 'ID already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		} else {
			errorId = ''
		}
		checking = false
	}

	async function forkWorkspace(): Promise<void> {
		try {
			// Use our new fork API endpoint
			const response = await fetch(`/api/w/${parentWorkspaceId}/workspaces/fork/create_fork`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					name,
					description: `Forked from ${parentWorkspace?.name || parentWorkspaceId}`
				})
			})

			if (!response.ok) {
				const error = await response.text()
				throw new Error(error)
			}

			const result = await response.json()
			
			// Update workspace color if specified
			if (colorEnabled && workspaceColor) {
				try {
					await fetch(`/api/w/${result.fork_workspace_id}/workspaces/change_workspace_color`, {
						method: 'POST',
						headers: {
							'Content-Type': 'application/json',
						},
						body: JSON.stringify({ color: workspaceColor })
					})
				} catch (colorError) {
					console.warn('Failed to set workspace color:', colorError)
				}
			}

			// Create user in the new workspace if needed
			if (!automateUsernameCreation && username) {
				try {
					await fetch(`/api/w/${result.fork_workspace_id}/workspaces/add_user`, {
						method: 'POST',
						headers: {
							'Content-Type': 'application/json',
						},
						body: JSON.stringify({ 
							email: $userStore?.email, 
							username,
							is_admin: true 
						})
					})
				} catch (userError) {
					console.warn('Failed to set username:', userError)
				}
			}

			sendUserToast(`Forked workspace ${result.fork_workspace_id} from ${parentWorkspaceId}`)

			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			switchWorkspace(result.fork_workspace_id)

			goto(rd ?? '/')
		} catch (error) {
			sendUserToast(`Failed to fork workspace: ${error}`, true)
		}
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			forkWorkspace()
		}
	}

	async function loadWorkspaces() {
		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			} catch {}
		}
		if (!$usersWorkspaceStore) {
			const url = $page.url
			console.log('logout 2')
			await logoutWithRedirect(url.href.replace(url.origin, ''))
		}
	}

	async function loadParentWorkspace() {
		try {
			// For now, we'll find it from the user's workspaces
			// In a real implementation, we might want to fetch workspace details
			const workspaces = $usersWorkspaceStore || await WorkspaceService.listUserWorkspaces()
			parentWorkspace = workspaces.workspaces.find(w => w.id === parentWorkspaceId) || 
				{ id: parentWorkspaceId, name: parentWorkspaceId }
			
			// Set default values based on parent workspace
			name = `${parentWorkspace.name} (Fork)`
		} catch {
			parentWorkspace = { id: parentWorkspaceId, name: parentWorkspaceId }
			name = `${parentWorkspaceId} (Fork)`
		}
	}

	let automateUsernameCreation = $state(false)
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? false

		if (!automateUsernameCreation) {
			UserService.globalWhoami().then((x) => {
				let uname = ''
				if (x.name) {
					uname = x.name.split(' ')[0]
				} else {
					uname = x.email.split('@')[0]
				}
				uname = uname.replace(/\./gi, '')
				username = uname.toLowerCase()
			})
		}
	}

	onMount(() => {
		loadWorkspaces()
		loadParentWorkspace()
		getAutomateUsernameCreationSetting()
	})

	run(() => {
		id = name.toLowerCase().replace(/\s/gi, '-')
	})
	run(() => {
		validateName(id)
	})
	run(() => {
		errorUser = validateUsername(username)
	})
	run(() => {
		colorEnabled && !workspaceColor && generateRandomColor()
	})
</script>

<CenteredModal title="Fork Workspace">
	{#if parentWorkspace}
		<div class="mb-4 p-3 bg-surface-secondary rounded-md">
			<div class="text-sm text-secondary">Forking from:</div>
			<div class="font-medium">{parentWorkspace.name}</div>
			<div class="text-xs text-tertiary font-mono">{parentWorkspace.id}</div>
		</div>
	{/if}

	<label class="block pb-4 pt-4">
		<span class="text-secondary text-sm">Workspace name</span>
		<span class="ml-4 text-tertiary text-xs">Displayable name</span>
		<!-- svelte-ignore a11y_autofocus -->
		<input autofocus type="text" bind:value={name} />
	</label>
	<label class="block pb-4">
		<span class="text-secondary text-sm">Workspace ID</span>
		<span class="ml-10 text-tertiary text-xs">Slug to uniquely identify your workspace</span>
		{#if errorId}
			<span class="text-red-500 text-xs">{errorId}</span>
		{/if}
		<input type="text" bind:value={id} class:input-error={errorId != ''} />
	</label>
	<label class="block pb-4">
		<span class="text-secondary text-sm">Workspace color</span>
		<span class="ml-5 text-tertiary text-xs"
			>Color to identify the current workspace in the list of workspaces</span
		>
		<div class="flex items-center gap-2">
			<Toggle bind:checked={colorEnabled} options={{ right: 'Enable' }} />
			{#if colorEnabled}<input
					class="w-10"
					type="color"
					bind:value={workspaceColor}
					disabled={!colorEnabled}
				/>{/if}
			<input
				type="text"
				class="w-24 text-sm"
				bind:value={workspaceColor}
				disabled={!colorEnabled}
			/>
			<Button on:click={generateRandomColor} size="xs" disabled={!colorEnabled}>Random</Button>
		</div>
	</label>
	{#if !automateUsernameCreation}
		<label class="block pb-4">
			<span class="text-secondary text-sm">Your username in that workspace</span>
			<input type="text" bind:value={username} onkeyup={handleKeyUp} />
			{#if errorUser}
				<span class="text-red-500 text-xs">{errorUser}</span>
			{/if}
		</label>
	{/if}

	<div class="mt-4 p-3 bg-surface-secondary rounded-md">
		<div class="text-sm text-secondary mb-2">Fork Strategy</div>
		<div class="text-xs text-tertiary">
			The forked workspace will initially reference all resources from the parent workspace to save space. 
			When you modify any resource, it will be automatically copied to your fork.
		</div>
	</div>

	<div class="flex flex-wrap flex-row justify-between pt-10 gap-1">
		<Button variant="border" size="sm" href="{base}/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
		<Button
			disabled={checking ||
				errorId != '' ||
				!name ||
				(!automateUsernameCreation && (errorUser != '' || !username)) ||
				!id}
			on:click={forkWorkspace}
		>
			Fork workspace
		</Button>
	</div>
</CenteredModal>