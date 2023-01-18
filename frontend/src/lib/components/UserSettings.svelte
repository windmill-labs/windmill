<script lang="ts">
	import { usersWorkspaceStore } from '$lib/stores'

	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { UserService, SettingsService } from '$lib/gen'
	import { displayDate, sendUserToast, getToday, copyToClipboard, truncate } from '$lib/utils'
	import { faClipboard, faPlus } from '@fortawesome/free-solid-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { Button } from '$lib/components/common'
	import Icon from 'svelte-awesome'

	let newPassword: string | undefined
	let passwordError: string | undefined
	let version: string | undefined
	let tokens: TruncatedToken[]
	let newToken: string | undefined
	let newTokenLabel: string | undefined
	let newTokenExpiration: number | undefined
	let displayCreateToken = false
	let login_type = 'none'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	let drawer: Drawer

	export function openDrawer() {
		loadVersion()
		loadLoginType()
		listTokens()
		drawer?.openDrawer()
	}

	export function toggleDrawer() {
		drawer?.toggleDrawer()
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

	async function loadVersion(): Promise<void> {
		version = await SettingsService.backendVersion()
	}
	async function loadLoginType(): Promise<void> {
		login_type = (await UserService.globalWhoami()).login_type
	}

	async function createToken(): Promise<void> {
		newToken = undefined
		let date: Date | undefined
		if (newTokenExpiration) {
			date = new Date()
			date.setDate(date.getDate() + newTokenExpiration)
		}
		newToken = await UserService.createToken({
			requestBody: { label: newTokenLabel, expiration: date?.toISOString() } as NewToken
		})
		listTokens()
		displayCreateToken = false
	}

	async function listTokens(): Promise<void> {
		tokens = await UserService.listTokens()
	}

	async function deleteToken(tokenPrefix: string) {
		await UserService.deleteToken({ tokenPrefix })
		sendUserToast('Succesfully deleted token')
		listTokens()
	}
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="User Settings" on:close={drawer.toggleDrawer}>
		<div class="flex flex-col h-full">
			<div>
				<div class="text-xs pt-1 pb-2 text-gray-500 ">
					Windmill {version}
				</div>

				<h2 class="border-b mt-4">User info</h2>
				<div class="">
					{#if passwordError}
						<div class="text-red-600 text-2xs grow">{passwordError}</div>
					{/if}
					<div class="flex flex-col gap-2 w-full ">
						<div class="mt-4">
							<label class="block w-60 mb-2 text-gray-500">
								<div class="text-gray-700">email</div>
								<input
									type="text"
									disabled
									value={$usersWorkspaceStore?.email}
									class="input mt-1"
								/>
							</label>
							{#if login_type == 'password'}
								<label class="block w-120">
									<div class="text-gray-700">password</div>
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
									<Button size="sm" btnClasses="mt-4" on:click={setPassword}>Set password</Button>
								</label>
							{:else if login_type == 'github'}
								<span>Authentified through Github OAuth2. Cannot set a password.</span>
							{/if}
						</div>
					</div>
				</div>

				<div class="grid grid-cols-2 pt-8 pb-1">
					<h2 class="py-0 my-0 border-b">Tokens</h2>
					<div class="flex justify-end border-b pb-1">
						<Button
							size="sm"
							startIcon={{ icon: faPlus }}
							btnClasses={displayCreateToken ? 'hidden' : ''}
							on:click={() => {
								displayCreateToken = !displayCreateToken
								newToken = undefined
								newTokenExpiration = undefined
								newTokenLabel = undefined
							}}
						>
							Create token
						</Button>
					</div>
				</div>
				<div class="text-2xs text-gray-500 italic pb-6">
					Authentify to the Windmill API with access tokens.
				</div>

				<div>
					<div
						class="{newToken
							? ''
							: 'hidden'} border rounded-md mb-6 px-2 py-2 bg-green-50 flex flex-row flex-wrap "
					>
						<div>
							Added token: <button on:click={() => copyToClipboard(newToken ?? '')} class="inline"
								>{truncate(newToken ?? '', 10, '****')} <Icon data={faClipboard} />
							</button>
						</div>
						<div class="pt-1 text-xs ml-2">
							Make sure to copy your personal access token now. You won’t be able to see it again!
						</div>
					</div>

					<!-- Token creation interface -->
					<div
						class="{displayCreateToken
							? ''
							: 'hidden'} py-3 px-3 border rounded-md mb-6 bg-gray-50 min-w-min"
					>
						<h3 class="pb-3 font-semibold">Add new token</h3>
						<div class="flex flex-row flex-wrap gap-x-2 w-full justify-between">
							<div class="flex flex-col">
								<label for="label"
									>Label <span class="text-xs text-gray-500">(optional)</span></label
								>
								<input type="text" bind:value={newTokenLabel} />
							</div>
							<div class="flex flex-col ">
								<label for="expires"
									>Expires In<span class="text-xs text-gray-500">(optional)</span>
								</label>
								<select bind:value={newTokenExpiration}>
									<option value={undefined}>No expiration</option>
									<option value={7}>7d</option>
									<option value={30}>30d</option>
									<option value={90}>90d</option>
								</select>
							</div>
							<div class="flex items-end">
								<Button btnClasses="!mt-2" on:click={createToken}>New Token</Button>
							</div>
						</div>
					</div>
				</div>
				<div class="overflow-auto">
					<TableCustom>
						<tr slot="header-row">
							<th>prefix</th>
							<th>label</th>
							<th>expiration</th>
							<th />
						</tr>
						<tbody slot="body">
							{#if tokens && tokens.length > 0}
								{#each tokens as { token_prefix, expiration, label }}
									<tr>
										<td class="grow">{token_prefix}****</td>
										<td class="grow">{label ?? ''}</td>
										<td class="grow">{displayDate(expiration ?? '')}</td>
										<td class="grow"
											><button
												class="text-red-500 text-xs underline"
												on:click={() => {
													deleteToken(token_prefix)
												}}>Delete</button
											></td
										>
									</tr>
								{/each}
							{:else if tokens && tokens.length === 0}
								<tr class="px-6"
									><td class="text-gray-700 italic text-xs"> There are no tokens yet</td></tr
								>
							{:else}
								<tr> Loading...</tr>
							{/if}
						</tbody>
					</TableCustom>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>
