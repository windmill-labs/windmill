<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { SettingService, UserService } from '$lib/gen'
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import { userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { parseMaxTokenExpirationDays } from '$lib/tokenExpiration'

	let port = $state(Number(page.url.searchParams.get('port')))
	let host: string = page.url.searchParams.get('host') || 'localhost'
	let scheme: string = page.url.searchParams.get('scheme') || 'http'
	port = untrack(() => port) == 0 || Number.isNaN(untrack(() => port)) ? 80 : untrack(() => port)

	const DEFAULT_DAYS = 30
	const CHOICES_DAYS = [1, 7, 30, 90, 365]
	const daysLabel = (days: number) => (days === 1 ? '1 day' : `${days} days`)

	// `max_token_expiration_days`: the server shortens a longer token itself, so the picker only
	// offers what it would keep. A service account is exempt from it, as on the token form.
	let maxExpirationDays = $state<number | undefined>(undefined)
	onMount(async () => {
		try {
			maxExpirationDays = parseMaxTokenExpirationDays(
				await SettingService.getGlobal({ key: 'max_token_expiration_days' })
			)
		} catch {}
	})
	const ceilingDays = $derived(
		($usersWorkspaceStore?.workspaces ?? []).some((w) => w.is_service_account)
			? undefined
			: maxExpirationDays
	)
	const expirationItems = $derived(
		ceilingDays == undefined
			? [
					...CHOICES_DAYS.map((d) => ({ label: daysLabel(d), value: d })),
					{ label: 'No expiration', value: 0 }
				]
			: [
					...CHOICES_DAYS.filter((d) => d < ceilingDays).map((d) => ({
						label: daysLabel(d),
						value: d
					})),
					{ label: `${daysLabel(ceilingDays)} (maximum)`, value: ceilingDays }
				]
	)
	// 0 stands for no expiration, since the Select clears to undefined.
	let pickedDays = $state<number | undefined>(undefined)
	const expirationDays = $derived.by(() => {
		const picked = pickedDays ?? DEFAULT_DAYS
		if (ceilingDays == undefined) return picked
		return picked === 0 || picked > ceilingDays ? ceilingDays : picked
	})

	async function authorizeToken(): Promise<void> {
		const username = $userStore?.username
		// `cli-login:` is reserved in `is_user_token` (windmill-common): the CLI signs in again on
		// its own once this token expires, so it must not email the user about it.
		const label = `cli-login:${username ?? ''}`
		const expiration =
			expirationDays === 0
				? undefined
				: new Date(Date.now() + expirationDays * 24 * 60 * 60 * 1000).toISOString()
		const newToken = await UserService.createToken({
			requestBody: {
				label,
				expiration
			}
		})

		const url =
			scheme + '://' + host + ':' + port + '?token=' + newToken + '&workspace=' + $workspaceStore
		window.location.href = url
	}
</script>

<CenteredModal title="Authorize login request" inPage>
	<p class="text-center text-lg mb-6">
		Token will be posted to {host == 'localhost' ? 'your local machine' : host} to port {port}
	</p>
	<div class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">Token expires in</span>
		<Select
			bind:value={() => expirationDays, (v) => (pickedDays = v)}
			inputClass="w-full"
			items={expirationItems}
		/>
		<p class="text-2xs text-hint">
			{#if ceilingDays != undefined}
				This instance limits tokens to {daysLabel(ceilingDays)}.
			{/if}
			Once it expires, the CLI asks you to sign in again.
		</p>
	</div>
	<div class="flex flex-row justify-around pt-4 gap-x-1">
		<Button variant="default" destructive unifiedSize="lg" href="/">Decline</Button>
		<Button variant="accent" unifiedSize="lg" onClick={authorizeToken}>Authorize</Button>
	</div>
</CenteredModal>
