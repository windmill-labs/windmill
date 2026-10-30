<script lang="ts" module>
	import type { AppAccessMode } from './appAccess'

	type AppLinks = {
		workspace: string
		path: string
		mode: AppAccessMode
		customPath: string | undefined
	}

	let shown: AppLinks | undefined = $state()

	/**
	 * Show the links of an app just created open to guests or to anyone, which reach it only
	 * through its secret or custom URL. Mounted once in the logged layout rather than in the editor,
	 * which the post-create navigation to the app's new path can remount.
	 */
	export function showAppLinks(links: AppLinks) {
		if (links.mode === 'publisher') return
		shown = links
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Label from '$lib/components/Label.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { AppService, SettingService } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'
	import { customUrlBaseOf, secretUrlOf } from './appAccess'

	let secretUrl: string | undefined = $state()
	let customUrl: string | undefined = $state()
	let error: string | undefined = $state()

	$effect(() => {
		const links = shown
		secretUrl = undefined
		customUrl = undefined
		error = undefined
		if (!links) return
		AppService.getPublicSecretOfApp({ workspace: links.workspace, path: links.path })
			.then((s) => {
				if (shown === links) secretUrl = secretUrlOf(links.workspace, s)
			})
			.catch((e) => {
				if (shown === links) error = `Could not read the secret URL: ${e?.body ?? e}`
			})
		if (links.customPath) {
			const customPath = links.customPath
			SettingService.getGlobal({ key: 'app_workspaced_route' })
				.then((s) => (s as boolean) ?? false)
				.catch(() => false)
				.then((workspaced) => {
					if (shown === links) customUrl = customUrlBaseOf(links.workspace, workspaced) + customPath
				})
		}
	})

	let name = $derived(shown?.path.split('/').pop() ?? '')
	let link = $derived(shown?.customPath ? 'one of these links' : 'this link')
</script>

<Modal
	bind:open={() => !!shown, (v) => (v ? undefined : (shown = undefined))}
	title="{name} is deployed"
	description={shown?.mode === 'anonymous'
		? `Anyone with ${link} can open it without signing in.`
		: `Anyone your identity provider authenticates can open it with ${link}, without a Windmill account.`}
	kind="X"
	class="sm:max-w-xl"
>
	<div class="flex flex-col gap-6">
		<Label label="Secret URL">
			{#if secretUrl}
				<ClipboardPanel content={secretUrl} size="md" />
			{:else if error}
				<span class="text-xs text-red-600 dark:text-red-400">{error}</span>
			{:else}
				<Loader2 size={16} class="animate-spin" />
			{/if}
		</Label>
		{#if shown?.customPath}
			<Label label="Custom URL">
				{#if customUrl}
					<ClipboardPanel content={customUrl} size="md" />
				{:else}
					<Loader2 size={16} class="animate-spin" />
				{/if}
			</Label>
		{/if}
		<span class="text-xs text-secondary">
			You can find {shown?.customPath ? 'both' : 'it'} again in Share.
		</span>
		<div class="flex flex-row justify-end">
			<Button variant="accent" unifiedSize="md" onclick={() => (shown = undefined)}>Done</Button>
		</div>
	</div>
</Modal>
