<script lang="ts">
	import { usersWorkspaceStore } from '$lib/stores'

	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { UserService, SettingsService } from '$lib/gen'
	import { displayDate, sendUserToast, getToday } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'

	let newPassword: string | undefined
	let passwordError: string | undefined
	let version: string | undefined
	let tokens: TruncatedToken[]
	let newToken: string | undefined
	let newTokenLabel: string | undefined
	let newTokenExpiration: string | undefined
	let displayCreateToken = false
	let login_type = 'none'

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
		let expirationISO: Date | undefined
		if (newTokenExpiration) {
			expirationISO = new Date(newTokenExpiration)
		}
		newToken = await UserService.createToken({
			requestBody: { label: newTokenLabel, expiration: expirationISO?.toISOString() } as NewToken
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

	loadVersion()
	loadLoginType()
	listTokens()
</script>

<CenteredModal title="User settings">
	<div class="flex flex-row justify-between">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
	</div>
	<div class="text-2xs text-gray-500 italic pb-6">
		Running windmill version (backend) {version}
	</div>
	<h2 class="border-b">User info</h2>
	<div class="">
		{#if passwordError}
			<div class="text-red-600 text-2xs grow">{passwordError}</div>
		{/if}
		<div class="flex flex-col gap-2 w-full ">
			<div class="mt-4">
				<label class="block w-60 mb-2 text-gray-500">
					<div class="text-gray-700">email</div>
					<input disabled value={$usersWorkspaceStore?.email} class="input mt-1" />
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

	<div class="grid grid-cols-2 pt-24 pb-1">
		<h2 class="py-0 my-0 border-b">Tokens</h2>
		<div class="flex justify-end border-b pb-1">
			<Button
				size="xs"
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
		<!-- Token creation notification -->
		<div
			class="{newToken
				? ''
				: 'hidden'} border rounded-md mb-6 px-2 py-2 bg-green-50 flex flex-row gap-12"
		>
			<div>
				Added token: <pre class="inline">{newToken}</pre>
			</div>
			<div class="pt-1 text-xs text-right">
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
			<div class="flex flex-col md:flex-row md:gap-24">
				<div class="flex flex-col container-bacf979e">
					<label for="expires">Expires on:</label>
					<input
						class="block md:w-1/2"
						type="date"
						id="expires"
						name="expiration-date"
						bind:value={newTokenExpiration}
						min={getToday().getFullYear() +
							'/' +
							getToday().getMonth() +
							'/' +
							getToday().getDate()}
						max={getToday().getFullYear() +
							1 +
							'/' +
							getToday().getMonth() +
							'/' +
							getToday().getDate()}
					/>
				</div>
				<label class="flex flex-col justify-end md:w-1/2"
					>Label (optional)<input bind:value={newTokenLabel} /></label
				>
				<div class="flex items-end">
					<Button size="sm" btnClasses="!mt-2" on:click={createToken}>Submit</Button>
				</div>
			</div>
		</div>

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

	<div class="flex flex-row justify-between pt-4">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
	</div>
</CenteredModal>
