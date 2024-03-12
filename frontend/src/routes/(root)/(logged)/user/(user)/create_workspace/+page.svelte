<script lang="ts">
	import { goto } from '$app/navigation'
	import { ResourceService, SettingService, UserService, WorkspaceService } from '$lib/gen'
	import { validateUsername } from '$lib/utils'
	import { logoutWithRedirect } from '$lib/logout'
	import { page } from '$app/stores'
	import { usersWorkspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import TestOpenaiKey from '$lib/components/copilot/TestOpenaiKey.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { isCloudHosted } from '$lib/cloud'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	const rd = $page.url.searchParams.get('rd')

	let id = ''
	let name = ''
	let username = ''

	let errorId = ''
	let errorUser = ''
	let openAiKey = ''
	let codeCompletionEnabled = true
	let checking = false

	$: id = name.toLowerCase().replace(/\s/gi, '-')

	$: validateName(id)
	$: errorUser = validateUsername(username)

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

	async function createWorkspace(): Promise<void> {
		await WorkspaceService.createWorkspace({
			requestBody: {
				id,
				name,
				username: automateUsernameCreation ? undefined : username
			}
		})
		if (auto_invite) {
			await WorkspaceService.editAutoInvite({
				workspace: id,
				requestBody: { operator: operatorOnly, invite_all: !isCloudHosted(), auto_add: autoAdd }
			})
		}
		if (openAiKey != '') {
			let actualUsername = username
			if (automateUsernameCreation) {
				const user = await UserService.whoami({
					workspace: id
				})
				actualUsername = user.username
			}
			let path = `u/${actualUsername}/openai_windmill_codegen`
			await ResourceService.createResource({
				workspace: id,
				requestBody: {
					path,
					value: {
						api_key: openAiKey
					},
					resource_type: 'openai'
				}
			})
			await WorkspaceService.editCopilotConfig({
				workspace: id,
				requestBody: { openai_resource_path: path, code_completion_enabled: codeCompletionEnabled }
			})
		}

		sendUserToast(`Created workspace id: ${id}`)

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		switchWorkspace(id)

		goto(rd ?? '/')
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			createWorkspace()
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
			await logoutWithRedirect(url.href.replace(url.origin, ''))
		}
	}

	let automateUsernameCreation = false
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			(await SettingService.getGlobal({ key: 'automate_username_creation' })) ?? false

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
	getAutomateUsernameCreationSetting()

	onMount(() => {
		loadWorkspaces()

		WorkspaceService.isDomainAllowed().then((x) => {
			isDomainAllowed = x
		})
	})

	let isDomainAllowed: undefined | boolean = undefined

	$: domain = $usersWorkspaceStore?.email.split('@')[1]

	let auto_invite = false
	let operatorOnly = false
	let autoAdd = false
</script>

<CenteredModal title="New Workspace">
	<label class="block pb-4 pt-4">
		<span class="text-secondary text-sm">Workspace name</span>
		<span class="ml-4 text-tertiary text-xs">Displayable name</span>
		<!-- svelte-ignore a11y-autofocus -->
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
	{#if !automateUsernameCreation}
		<label class="block pb-4">
			<span class="text-secondary text-sm">Your username in that workspace</span>
			<input type="text" bind:value={username} on:keyup={handleKeyUp} />
			{#if errorUser}
				<span class="text-red-500 text-xs">{errorUser}</span>
			{/if}
		</label>
	{/if}
	<label class="block pb-4">
		<span class="text-secondary text-sm">
			OpenAI key for Windmill AI
			<Tooltip>
				Find out how it can help you <a
					href="https://www.windmill.dev/docs/core_concepts/ai_generation"
					target="_blank"
					rel="noopener noreferrer">in the docs</a
				>
			</Tooltip>
			<span class="text-2xs text-tertiary ml-2">(optional but recommended)</span>
		</span>
		<div class="flex flex-row gap-1">
			<input type="password" bind:value={openAiKey} on:keyup={handleKeyUp} />
			<TestOpenaiKey apiKey={openAiKey} disabled={!openAiKey} />
		</div>
		{#if openAiKey}
			<Toggle
				disabled={!openAiKey}
				size="xs"
				bind:checked={codeCompletionEnabled}
				options={{ right: 'Enable code completion' }}
			/>
		{/if}
	</label>
	<Toggle
		disabled={isCloudHosted() && !isDomainAllowed}
		bind:checked={auto_invite}
		options={{
			right: isCloudHosted()
				? `Auto-invite anyone from ${domain}`
				: `Auto-invite anyone joining the instance`
		}}
	/>
	{#if isCloudHosted() && isDomainAllowed == false}
		<div class="text-tertiary text-sm mb-4 mt-2">{domain} domain not allowed for auto-invite</div>
	{/if}
	<div class={'overflow-hidden transition-all ' + (auto_invite ? 'h-36' : 'h-0')}>
		<div class="text-xs mb-1 leading-6 pt-2">
			Mode <Tooltip>Whether to invite or add users directly to the workspace.</Tooltip>
		</div>
		<ToggleButtonGroup
			selected={autoAdd ? 'add' : 'invite'}
			on:selected={(e) => {
				autoAdd = e.detail == 'add'
			}}
		>
			<ToggleButton value="invite" size="xs" label="Auto-invite" />
			<ToggleButton value="add" size="xs" label="Auto-add" />
		</ToggleButtonGroup>

		<div class="text-xs mb-1 leading-6 pt-2"
			>Role <Tooltip>Role of the auto-invited users</Tooltip></div
		>
		<ToggleButtonGroup
			selected={operatorOnly ? 'operator' : 'developer'}
			on:selected={(e) => {
				operatorOnly = e.detail == 'operator'
			}}
		>
			<ToggleButton value="operator" size="xs" label="Operator" />
			<ToggleButton value="developer" size="xs" label="Developer" />
		</ToggleButtonGroup>
	</div>
	<div class="flex flex-wrap flex-row justify-between pt-10 gap-1">
		<Button variant="border" size="sm" href="/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
		<Button
			disabled={checking ||
				errorId != '' ||
				!name ||
				(!automateUsernameCreation && (errorUser != '' || !username)) ||
				!id}
			on:click={createWorkspace}
		>
			Create workspace
		</Button>
	</div>
</CenteredModal>
