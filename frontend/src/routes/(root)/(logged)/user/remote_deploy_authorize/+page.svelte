<script lang="ts">
	import { page } from '$app/state'
	import { base } from '$lib/base'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Alert, Button } from '$lib/components/common'
	import { UserService } from '$lib/gen'
	import { CALLBACK_PATH } from '$lib/remoteDeploy'
	import { usersWorkspaceStore } from '$lib/stores'

	// Another Windmill instance asks for a token to deploy into one workspace here as the signed-in
	// user. It gets the token only if they authorize, and only at `callback`, whose origin they are
	// shown: anyone can craft this link, so that origin is the whole decision.
	const workspace = page.url.searchParams.get('workspace') ?? ''
	const connectState = page.url.searchParams.get('state') ?? ''
	const callback = parseCallback(page.url.searchParams.get('callback'))

	function parseCallback(raw: string | null): URL | undefined {
		try {
			const url = new URL(raw ?? '')
			const valid =
				(url.protocol === 'https:' || url.protocol === 'http:') &&
				url.pathname.endsWith(CALLBACK_PATH) &&
				!url.search &&
				!url.hash &&
				!url.username &&
				!url.password
			return valid ? url : undefined
		} catch {
			return undefined
		}
	}

	// A framing page could lay its own content over the Authorize button and have it clicked.
	const framed = window.self !== window.top
	// Left to the user rather than refused: internal instances often serve plain http.
	const unencrypted =
		callback?.protocol === 'http:' &&
		!['localhost', '127.0.0.1', '[::1]'].includes(callback.hostname)
	// Bounds a grant that went to the wrong place; reconnecting is one click.
	const TOKEN_LIFETIME_MS = 90 * 24 * 3600 * 1000
	const invalid = !callback || !workspace || !/^[A-Za-z0-9]{8,128}$/.test(connectState)

	let error: string | undefined = $state(undefined)
	let authorizing = $state(false)

	function sendBack(params: Record<string, string>) {
		const fragment = new URLSearchParams({ ...params, state: connectState })
		window.location.replace(`${callback!.href}#${fragment}`)
	}

	async function authorize() {
		authorizing = true
		error = undefined
		try {
			await UserService.whoami({ workspace })
			const token = await UserService.createToken({
				requestBody: {
					label: `remote-deploy:${callback!.host}`,
					workspace_id: workspace,
					expiration: new Date(Date.now() + TOKEN_LIFETIME_MS).toISOString()
				}
			})
			sendBack({ token })
		} catch (e: any) {
			authorizing = false
			error =
				e?.status === 401 || e?.status === 404
					? `You are not a member of workspace ${workspace} on this instance`
					: (e?.body ?? e?.message ?? String(e))
		}
	}
</script>

<CenteredModal title="Authorize a deploy connection">
	{#if framed}
		<Alert type="error" title="Refused">
			This page cannot be used inside another page. Open it in its own tab.
		</Alert>
	{:else if invalid}
		<Alert type="error" title="Invalid request">
			This authorization link is malformed. Start again from the deploy drawer of the other
			instance.
		</Alert>
	{:else}
		<div class="flex flex-col gap-4 text-sm text-primary">
			<p>
				<span class="font-semibold">{callback!.origin}</span> asks to deploy into workspace
				<span class="font-semibold">{workspace}</span> on this instance as
				<span class="font-semibold">{$usersWorkspaceStore?.email ?? 'you'}</span>.
			</p>
			<p class="text-secondary">
				It will receive a token limited to that workspace, with your permissions in it, valid for up
				to 90 days. Only authorize if you just started this from {callback!.host}. You can revoke
				the token at any time in your account settings, under tokens.
			</p>
			{#if unencrypted}
				<Alert type="warning" title="Unencrypted connection">
					{callback!.origin} is served over plain http, so the token will cross the network unencrypted
					on its way there.
				</Alert>
			{/if}
			{#if error}
				<Alert type="error" title="Could not authorize">{error}</Alert>
			{/if}
		</div>
		<div class="flex flex-row justify-around pt-6 gap-x-1">
			<Button
				variant="default"
				unifiedSize="lg"
				onclick={() => sendBack({ error: 'Declined on the remote instance' })}
			>
				Decline
			</Button>
			<Button
				variant="accent"
				unifiedSize="lg"
				loading={authorizing}
				disabled={authorizing}
				onclick={authorize}
			>
				Authorize
			</Button>
		</div>
	{/if}
	{#if framed || invalid}
		<div class="flex justify-center pt-4">
			<Button variant="default" unifiedSize="md" href={base}>Go to Windmill</Button>
		</div>
	{/if}
</CenteredModal>
