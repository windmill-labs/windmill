<script lang="ts" module>
	export type ShareKind = 'flow' | 'script' | 'app' | 'raw_app'
	export type ShareSection = 'app'
</script>

<script lang="ts">
	import { tick } from 'svelte'
	import { Alert, Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import ItemAccessList from './ItemAccessList.svelte'
	import AppAccessSection from './AppAccessSection.svelte'
	import {
		accessModeOf,
		canPreserveOnBehalfOf,
		updateDeployedAppAccess,
		type AppAccessMode
	} from './appAccess'
	import Label from '$lib/components/Label.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { logShareOpen, type ShareOpenSource } from './sharingTelemetry'
	import { useEditRights } from '$lib/operatorWriteRights'

	interface Props {
		/** Called after a write to the app row, with the row as it now is. */
		onAppChanged?: (row: AppWithLastVersion) => void
		/** Called after any change, for a list to refresh. */
		onChange?: () => void
	}

	let { onAppChanged, onChange }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const editRights = useEditRights()

	let open = $state(false)
	let kind = $state<ShareKind>('flow')
	let path = $state('')
	/** Opened from an editor, whose other settings wait for the next deploy. */
	let fromEditor = $state(false)

	let app: AppWithLastVersion | undefined = $state()
	let appBusy = $state(false)

	let name = $derived(path.split('/').pop() ?? path)
	let noun = $derived(kind === 'raw_app' ? 'app' : kind)
	let isApp = $derived(kind === 'app' || kind === 'raw_app')
	let canPreserve = $derived(canPreserveOnBehalfOf(operatingUser.current))
	let runAs = $derived(app?.policy.on_behalf_of_email ?? app?.policy.on_behalf_of)
	// Who can open the app is part of its policy, which only someone who can edit the app may
	// rewrite; the lists and detail pages open this dialog for readers too.
	let canWriteApp = $derived(
		!!app && editRights.canEditItem(app.raw_app ? 'raw_app' : 'app', app.path, app.extra_perms)
	)
	let itemUrl = $derived(
		`${window.location.origin}/${kind === 'flow' ? 'flows' : 'scripts'}/get/${path}?workspace=${$operatingWorkspace}`
	)

	export async function openDialog(
		kind_l: ShareKind,
		path_l: string,
		opts: { source: ShareOpenSource; section?: ShareSection; fromEditor?: boolean }
	) {
		logShareOpen(opts.source)
		kind = kind_l
		path = path_l
		fromEditor = opts.fromEditor ?? false
		app = undefined
		open = true
		if (kind === 'app' || kind === 'raw_app') {
			void loadApp()
		}
		if (opts.section) {
			await tick()
			document.getElementById(`share-section-${opts.section}`)?.scrollIntoView({ block: 'nearest' })
		}
	}

	async function loadApp() {
		const asked = path
		const row = await AppService.getAppLiteByPath({ workspace: $operatingWorkspace!, path })
		if (path === asked) app = row
	}

	async function changeApp(
		change: { execution_mode: AppAccessMode } | { custom_path: string | null }
	) {
		appBusy = true
		try {
			app = await updateDeployedAppAccess($operatingWorkspace!, path, change)
			onAppChanged?.(app)
			onChange?.()
		} catch (err) {
			sendUserToast(`Could not change the app's access: ${err?.body ?? err}`, true)
			// Re-read so the controls show what is deployed, not the choice that was refused.
			await loadApp()
		} finally {
			appBusy = false
		}
	}
</script>

<Modal bind:open title="Share {name}" kind="X" enterConfirms={false} fitHeight class="sm:max-w-xl">
	<!-- A long access list gives up height when the sections below grow, and keeps the dialog
	     inside the viewport. -->
	<div class="flex flex-col gap-6 h-full">
		{#key path}
			<ItemAccessList {kind} {path} {onChange} />
		{/key}

		{#if isApp}
			<section id="share-section-app" class="flex flex-col gap-2">
				<h3 class="text-sm font-semibold text-emphasis">Who can open the app</h3>
				{#if app}
					{#if !canWriteApp}
						<span class="text-xs text-secondary">
							Only people who can edit this app can change who can open it.
						</span>
					{:else if app.policy.on_behalf_of && !canPreserve}
						<Alert type="info" size="xs" title="The app will run as you">
							Changing who can open it redeploys its policy as you, so its runnables will run as you
							instead of {runAs}.
						</Alert>
					{/if}
					{#key path}
						<AppAccessSection
							workspace={$operatingWorkspace!}
							appPath={path}
							rawApp={kind === 'raw_app' || app.raw_app}
							mode={accessModeOf(app.policy)}
							onModeChange={(m) => changeApp({ execution_mode: m })}
							deployed={{
								customPath: app.custom_path ?? undefined,
								onCustomPathChange: (p) => changeApp({ custom_path: p })
							}}
							disabled={appBusy || !canWriteApp}
						/>
					{/key}
				{:else}
					<span class="text-xs text-secondary">Loading…</span>
				{/if}
			</section>
		{:else}
			<section class="flex flex-col gap-2">
				<h3 class="text-sm font-semibold text-emphasis">Who can open the {noun}</h3>
				<span class="text-xs text-secondary">
					The people with access above, signed in to Windmill.
				</span>
				<Label label="Link">
					<ClipboardPanel content={itemUrl} size="md" />
				</Label>
			</section>
		{/if}

		<div class="flex flex-row items-center gap-2">
			<!-- From an editor, where every other change waits for Deploy, say which version this is. -->
			<span class="grow text-xs text-secondary">
				{fromEditor
					? `Changes apply to the deployed ${noun} right away.`
					: 'Changes apply right away.'}
			</span>
			<Button variant="accent" unifiedSize="md" onclick={() => (open = false)}>Done</Button>
		</div>
	</div>
</Modal>
