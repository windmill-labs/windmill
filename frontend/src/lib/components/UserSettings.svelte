<script lang="ts">
	import {
		codeCompletionSessionEnabled,
		metadataCompletionEnabled,
		stepInputCompletionEnabled,
		usersWorkspaceStore
	} from '$lib/stores'
	import type { TruncatedToken } from '$lib/gen'
	import { UserService } from '$lib/gen'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { sendUserToast } from '$lib/toast'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
	import Toggle from './Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import McpTokensTable from './McpTokensTable.svelte'
	import TokensTable from './TokensTable.svelte'
	import { createEventDispatcher } from 'svelte'

	export let scopes: string[] | undefined = undefined
	export let newTokenLabel: string | undefined = undefined
	export let newTokenWorkspace: string | undefined = undefined
	export let newToken: string | undefined = undefined

	let newPassword: string | undefined
	let passwordError: string | undefined
	let tokens: TruncatedToken[]
	let login_type = 'none'
	let drawer: Drawer
	let mcpTokens: TruncatedToken[] = []
	let tokenPage = 1

	const dispatch = createEventDispatcher()

	export function openDrawer() {
		loadLoginType()
		listTokens()
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
		removeHash()
	}

	function removeHash() {
		window.location.hash = ''
	}

	async function setPassword(): Promise<void> {
		if (newPassword) {
			await UserService.setPassword({
				requestBody: {
					password: newPassword
				}
			})
			sendUserToast('Your password was successfully updated')
		} else {
			sendUserToast('Specify a new password value to change your passord', true)
		}
	}

	async function loadLoginType(): Promise<void> {
		login_type = (await UserService.globalWhoami()).login_type
	}

	async function listTokens(): Promise<void> {
		const allTokens = await UserService.listTokens({
			excludeEphemeral: true,
			page: tokenPage,
			perPage: 100
		})
		tokens = allTokens.filter(
			(token) => !token.scopes || token.scopes.find((scope) => !scope.startsWith('mcp:'))
		)
		mcpTokens = allTokens.filter(
			(token) => token.scopes && token.scopes.find((scope) => scope.startsWith('mcp:'))
		)
	}

	async function deleteToken(tokenPrefix: string) {
		await UserService.deleteToken({ tokenPrefix })
		sendUserToast('Succesfully deleted token')
		listTokens()
	}

	function loadSettings() {
		$codeCompletionSessionEnabled =
			(getLocalSetting('codeCompletionSessionEnabled') ?? 'true') == 'true'
		$metadataCompletionEnabled = (getLocalSetting('metadataCompletionEnabled') ?? 'true') == 'true'
		$stepInputCompletionEnabled =
			(getLocalSetting('stepInputCompletionEnabled') ?? 'true') == 'true'
	}

	function updateSetting(store: Writable<boolean>, value: boolean, setting: string) {
		store.set(value)
		storeLocalSetting(setting, value.toString())
	}

	function handleNextPage() {
		tokenPage += 1
		listTokens()
	}

	function handlePreviousPage() {
		tokenPage -= 1
		listTokens()
	}

	function handleTokenCreated(event: CustomEvent<string>) {
		newToken = event.detail
		dispatch('tokenCreated', newToken)
	}

	loadSettings()
</script>

<Drawer bind:this={drawer} size="800px" on:close={removeHash}>
	<DrawerContent title="User Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full">
			<div>
				{#if scopes == undefined}
					<div class="text-xs pt-1 pb-2 text-tertiary flex-col flex">
						Windmill <Version />
					</div>
					<div class="font-semibold flex items-baseline">
						Theme: <DarkModeToggle forcedDarkMode={false} />
					</div>
					<div class="flex flex-wrap md:gap-40 gap-10 mt-2">
						<div>
							<h2 class="border-b">User info</h2>
							<div class="">
								{#if passwordError}
									<div class="text-red-600 text-2xs grow">{passwordError}</div>
								{/if}
								<div class="flex flex-col gap-2 w-full">
									<div class="mt-4">
										<label class="block w-60 mb-2 text-tertiary">
											<div class="text-secondary">email</div>
											<input
												type="text"
												disabled
												value={$usersWorkspaceStore?.email}
												class="input mt-1"
											/>
										</label>
										{#if login_type == 'password'}
											<label class="block w-120">
												<div class="text-secondary">password</div>
												<input
													type="password"
													bind:value={newPassword}
													class="
							w-full
							block
							py-1
							px-2
							rounded-md
							border
							border-gray-300
							shadow-sm
							focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
							text-sm
							"
												/>
												<Button size="sm" btnClasses="mt-4 w-min" on:click={setPassword}
													>Set password</Button
												>
											</label>
										{:else if login_type == 'github'}
											<span>Authentified through Github OAuth2. Cannot set a password.</span>
										{/if}
									</div>
								</div>
							</div>
						</div>

						<div>
							<h2 class="border-b">AI user settings</h2>

							<div class="flex flex-col gap-4 mt-2">
								<Toggle
									on:change={(e) => {
										updateSetting(
											codeCompletionSessionEnabled,
											e.detail,
											'codeCompletionSessionEnabled'
										)
									}}
									checked={$codeCompletionSessionEnabled}
									options={{
										right: 'Code completion',
										rightTooltip: 'AI completion in the code editors'
									}}
								/>
								<Toggle
									on:change={(e) => {
										updateSetting(metadataCompletionEnabled, e.detail, 'metadataCompletionEnabled')
									}}
									checked={$metadataCompletionEnabled}
									options={{
										right: 'Metadata completion',
										rightTooltip: 'AI completion for summaries and descriptions'
									}}
								/>
								<Toggle
									on:change={(e) => {
										updateSetting(
											stepInputCompletionEnabled,
											e.detail,
											'stepInputCompletionEnabled'
										)
									}}
									checked={$stepInputCompletionEnabled}
									options={{
										right: 'Flow step input completion',
										rightTooltip: 'AI completion for flow step inputs'
									}}
								/>
							</div>
						</div>
					</div>
				{/if}

				<McpTokensTable tokens={mcpTokens} onDeleteToken={deleteToken} onListTokens={listTokens} />

				<TokensTable
					{tokens}
					defaultNewTokenLabel={newTokenLabel}
					defaultNewTokenWorkspace={newTokenWorkspace}
					onDeleteToken={deleteToken}
					{tokenPage}
					onNextPage={handleNextPage}
					onPreviousPage={handlePreviousPage}
					onListTokens={listTokens}
					{scopes}
					on:tokenCreated={handleTokenCreated}
				/>
			</div>
		</div>
	</DrawerContent>
</Drawer>
