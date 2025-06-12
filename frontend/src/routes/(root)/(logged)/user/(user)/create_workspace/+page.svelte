<script lang="ts">
	import { run } from 'svelte/legacy'

	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import {
		ResourceService,
		SettingService,
		UserService,
		VariableService,
		WorkspaceService,
		type AIProvider
	} from '$lib/gen'
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
	import TestAIKey from '$lib/components/copilot/TestAIKey.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { isCloudHosted } from '$lib/cloud'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { AI_DEFAULT_MODELS } from '$lib/components/copilot/lib'

	const rd = $page.url.searchParams.get('rd')

	let id = $state('')
	let name = $state('')
	let username = $state('')

	let errorId = $state('')
	let errorUser = $state('')
	let aiKey = $state('')
	let codeCompletionEnabled = $state(true)
	let checking = $state(false)

	let workspaceColor: string | null = $state(null)
	let colorEnabled = $state(false)

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

	async function createWorkspace(): Promise<void> {
		await WorkspaceService.createWorkspace({
			requestBody: {
				id,
				name,
				color: colorEnabled && workspaceColor ? workspaceColor : undefined,
				username: automateUsernameCreation ? undefined : username
			}
		})
		if (auto_invite) {
			await WorkspaceService.editAutoInvite({
				workspace: id,
				requestBody: { operator: operatorOnly, invite_all: !isCloudHosted(), auto_add: true }
			})
		}
		if (aiKey != '') {
			let actualUsername = username
			if (automateUsernameCreation) {
				const user = await UserService.whoami({
					workspace: id
				})
				actualUsername = user.username
			}
			let path = `u/${actualUsername}/${selected}_windmill_codegen`
			await VariableService.createVariable({
				workspace: id,
				requestBody: {
					path,
					value: aiKey,
					is_secret: true,
					description: 'Ai token'
				}
			})
			await ResourceService.createResource({
				workspace: id,
				requestBody: {
					path,
					value: {
						api_key: '$var:' + path
					},
					resource_type: selected
				}
			})
			await WorkspaceService.editCopilotConfig({
				workspace: id,
				requestBody: aiKey
					? {
							providers: {
								[selected]: {
									resource_path: path,
									models: [AI_DEFAULT_MODELS[selected][0]]
								}
							},
							default_model: {
								model: AI_DEFAULT_MODELS[selected][0],
								provider: selected
							},
							code_completion_model: codeCompletionEnabled
								? { model: AI_DEFAULT_MODELS[selected][0], provider: selected }
								: undefined
						}
					: {}
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
	getAutomateUsernameCreationSetting()

	onMount(() => {
		loadWorkspaces()

		WorkspaceService.isDomainAllowed().then((x) => {
			isDomainAllowed = x
		})
	})

	let isDomainAllowed: undefined | boolean = $state(undefined)

	let auto_invite = $state(false)
	let operatorOnly = $state(false)
	let selected: Exclude<AIProvider, 'customai'> = $state('openai')
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
	let domain = $derived($usersWorkspaceStore?.email.split('@')[1])
</script>

<CenteredModal title="New Workspace">
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
	<div class="block pb-4">
		<label for="ai-key" class="flex flex-col gap-1">
			<span class="text-secondary text-sm">
				AI key for Windmill AI
				<Tooltip>
					Find out how it can help you <a
						href="https://www.windmill.dev/docs/core_concepts/ai_generation"
						target="_blank"
						rel="noopener noreferrer">in the docs</a
					>
				</Tooltip>
				<span class="text-2xs text-tertiary ml-2">(optional but recommended)</span>
			</span>

			<div class="pb-2">
				<ToggleButtonGroup bind:selected>
					{#snippet children({ item })}
						<ToggleButton value="openai" label="OpenAI" {item} />
						<ToggleButton value="anthropic" label="Anthropic" {item} />
						<ToggleButton value="mistral" label="Mistral" {item} />
						<ToggleButton value="deepseek" label="DeepSeek" {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>
		</label>

		<div class="flex flex-row gap-1 pb-4">
			<input
				id="ai-key"
				type="password"
				autocomplete="new-password"
				bind:value={aiKey}
				onkeyup={handleKeyUp}
			/>
			<TestAIKey
				apiKey={aiKey}
				disabled={!aiKey}
				aiProvider={selected}
				model={AI_DEFAULT_MODELS[selected][0]}
			/>
		</div>
		{#if aiKey}
			<Toggle
				disabled={!aiKey}
				bind:checked={codeCompletionEnabled}
				options={{ right: 'Enable code completion' }}
			/>
		{/if}
	</div>
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

		<div class="text-xs mb-1 leading-6 pt-2"
			>Role <Tooltip>Role of the auto-invited users</Tooltip></div
		>
		<ToggleButtonGroup
			selected={operatorOnly ? 'operator' : 'developer'}
			on:selected={(e) => {
				operatorOnly = e.detail == 'operator'
			}}
		>
			{#snippet children({ item })}
				<ToggleButton value="operator" size="xs" label="Operator" {item} />
				<ToggleButton value="developer" size="xs" label="Developer" {item} />
			{/snippet}
		</ToggleButtonGroup>
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
			on:click={createWorkspace}
		>
			Create workspace
		</Button>
	</div>
</CenteredModal>
