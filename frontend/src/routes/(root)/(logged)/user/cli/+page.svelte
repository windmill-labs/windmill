<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { SettingService, UserService } from '$lib/gen'
	import { page } from '$app/state'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { superadmin, userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
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

	// Instance-wide by default: `wmill workspace add`, `workspace list/fork` and `instance`
	// commands call instance-level endpoints, which a workspace-bound token is refused on. The
	// owner's admin and superadmin rights are carried only when asked for.
	// Binding is offered only for the workspace the CLI names (`&workspace=`): the CLI files the
	// token under that profile, so a token bound to any other workspace would be refused there.
	// An older CLI, or `wmill instance`, names none and gets an instance-wide token.
	const cliWorkspace = page.url.searchParams.get('workspace') || undefined
	let reach = $state<'workspace' | 'all'>('all')
	let carryAdmin = $state(false)
	let carrySuperAdmin = $state(false)
	const isAdminSomewhere = $derived(!!$userStore?.is_admin || !!$superadmin)

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
				expiration,
				workspace_id: reach === 'workspace' ? cliWorkspace : undefined,
				admin: carryAdmin,
				super_admin: carrySuperAdmin
			}
		})

		const url =
			scheme +
			'://' +
			host +
			':' +
			port +
			'?token=' +
			newToken +
			'&workspace=' +
			(cliWorkspace ?? $workspaceStore)
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
	{#if cliWorkspace}
		<div class="flex flex-col gap-1 pt-4">
			<span class="text-xs font-semibold text-emphasis">Works in</span>
			<ToggleButtonGroup bind:selected={reach} allowEmpty={false}>
				{#snippet children({ item })}
					<ToggleButton {item} value="all" label="All workspaces (instance-wide)" />
					<ToggleButton {item} value="workspace" label={`Only ${cliWorkspace}`} />
				{/snippet}
			</ToggleButtonGroup>
			<p class="text-2xs text-hint">
				{#if reach === 'all'}
					Needed for <code>wmill workspace add</code>, <code>workspace list</code>/<code>fork</code>
					and <code>instance</code> commands.
				{:else}
					Enough to sync {cliWorkspace}; commands that reach other workspaces or the instance are
					refused.
				{/if}
			</p>
		</div>
	{/if}
	{#if (isAdminSomewhere && !carrySuperAdmin) || $superadmin}
		<div class="flex flex-col gap-2 pt-4">
			{#if isAdminSomewhere && !carrySuperAdmin}
				<Toggle
					bind:checked={carryAdmin}
					options={{
						right: 'Workspace admin rights',
						rightTooltip:
							'Needed to sync workspace settings, users and groups. Off, the CLI acts as a regular member of your workspaces.'
					}}
					size="xs"
				/>
			{/if}
			{#if $superadmin}
				<Toggle
					bind:checked={carrySuperAdmin}
					options={{
						right: 'Superadmin rights',
						rightTooltip:
							'Needed for instance commands such as `wmill instance push`. Off, the CLI cannot reach instance settings or workspaces you are not a member of.'
					}}
					size="xs"
				/>
			{/if}
		</div>
	{/if}
	<div class="flex flex-row justify-around pt-4 gap-x-1">
		<Button variant="default" destructive unifiedSize="lg" href="/">Decline</Button>
		<Button
			variant="accent"
			unifiedSize="lg"
			onClick={authorizeToken}>Authorize</Button
		>
	</div>
</CenteredModal>
