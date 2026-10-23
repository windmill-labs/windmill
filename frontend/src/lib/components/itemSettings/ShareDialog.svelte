<script lang="ts" module>
	export type ShareKind = 'flow' | 'script' | 'app' | 'raw_app'
</script>

<script lang="ts">
	import { Alert, Button, Section } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import {
		AppService,
		FlowService,
		FolderService,
		GranularAclService,
		GroupService,
		ScriptService,
		type AppWithLastVersion,
		type Folder
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import ItemAccessList from './ItemAccessList.svelte'
	import AppAccessSection from './AppAccessSection.svelte'
	import {
		accessModeOf,
		modeChangeRefusal,
		updateDeployedAppAccess,
		type AppAccessMode
	} from './appAccess'
	import { canPreserveOnBehalfOf } from './onBehalfOf'
	import {
		accessLostByMove,
		folderOfPath,
		inheritedAccess,
		movesToAnotherOwner,
		ownerLabel
	} from './inheritedAccess'
	import Label from '$lib/components/Label.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { logShareOpen, type ShareOpenSource } from './sharingTelemetry'
	import { useEditRights, useRoleCapabilities } from '$lib/operatorWriteRights'

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
	const roleCaps = useRoleCapabilities()

	let open = $state(false)
	let kind = $state<ShareKind>('flow')
	let path = $state('')
	/** Opened from an editor, whose other settings wait for the next deploy. */
	let fromEditor = $state(false)
	/** Where the editor's draft moves the item, when that is under another owner. */
	let moveTo: string | undefined = $state()
	/** Who that move takes access from, once read. */
	let moveLoss: { lost: string[]; demoted: string[] } | undefined = $state()
	let moveLossReads = 0
	/** The label of the button that goes on with a deploy waiting on this dialog. */
	let deployLabel: string | undefined = $state()
	let answerDeploy: ((deploy: boolean) => void) | undefined = undefined

	let app: AppWithLastVersion | undefined = $state()
	let appBusy = $state(false)
	/** Who the deployed flow or script runs as, and whether the user may change it, once read. An
	 *  app's run-as user is in its policy. */
	let runnableIdentity: { runAs: string | undefined; canEdit: boolean } | undefined = $state()

	let name = $derived(path.split('/').pop() ?? path)
	let noun = $derived(kind === 'raw_app' ? 'app' : kind)
	let isApp = $derived(kind === 'app' || kind === 'raw_app')
	let runAs = $derived(app?.policy.on_behalf_of ?? app?.policy.on_behalf_of_email)
	// Who can open the app is part of its policy, which only someone who can edit the app may
	// rewrite; the lists and detail pages open this dialog for readers too.
	let canWriteApp = $derived(
		!!app && editRights.canEditItem(app.raw_app ? 'raw_app' : 'app', app.path, app.extra_perms)
	)
	let modeRefusal = $derived(
		app &&
			modeChangeRefusal(app.policy, {
				username: operatingUser.current?.username,
				canPreserveRunAs: canPreserveOnBehalfOf(operatingUser.current),
				composeOnly: $roleCaps.apps === 'compose'
			})
	)
	let itemUrl = $derived(
		`${window.location.origin}/${kind === 'flow' ? 'flows' : 'scripts'}/get/${path}?workspace=${$operatingWorkspace}`
	)
	// What a run can reach. It changes only with a deploy, so Share explains it and says where.
	let permissions = $derived.by(() => {
		// A viewer-mode app runs its runnables as whoever opens it, whatever its run-as user.
		const identity = isApp
			? app && {
					runAs: app.policy.execution_mode === 'viewer' ? undefined : runAs,
					canEdit: canWriteApp
				}
			: runnableIdentity
		if (!identity) return undefined
		const who = identity.runAs
		return {
			summary: who
				? `Uses the access of ${who}`
				: `Uses the access of whoever ${isApp ? 'opens' : 'runs'} it`,
			body: !who
				? isApp
					? 'Its runnables can use only the resources and variables that the person who opens the app can access.'
					: 'A run can use only the resources and variables that the person who starts it can access.'
				: isApp
					? `Its runnables can use the resources and variables ${who} can access, for everyone who can open the app.`
					: `Every run can use the resources and variables ${who} can access, whoever starts it. Who started a run stays recorded, and the code can read their email from WM_END_USER_EMAIL.`,
			canEdit: identity.canEdit
		}
	})

	/** `stagedPath` is the path the editor's draft deploys the item to. */
	export function openDialog(
		kind_l: ShareKind,
		path_l: string,
		opts: { source: ShareOpenSource; fromEditor?: boolean; stagedPath?: string }
	) {
		settleDeploy(false)
		logShareOpen(opts.source)
		kind = kind_l
		path = path_l
		fromEditor = opts.fromEditor ?? false
		moveTo =
			opts.stagedPath && movesToAnotherOwner(path_l, opts.stagedPath) ? opts.stagedPath : undefined
		moveLoss = undefined
		deployLabel = undefined
		app = undefined
		runnableIdentity = undefined
		open = true
		if (kind === 'app' || kind === 'raw_app') {
			void loadApp()
		} else {
			// Without it the section stays hidden: it only explains.
			void loadRunnableIdentity().catch(() => {})
		}
		if (moveTo) void loadMoveLoss()
	}

	/** Opens the dialog before a deploy that moves the item to another owner, for the deployer to
	 *  see who loses access and give it back first. Resolves whether to go on with the deploy. */
	export function confirmMove(
		kind_l: ShareKind,
		path_l: string,
		stagedPath: string,
		label: string
	): Promise<boolean> {
		openDialog(kind_l, path_l, { source: 'deploy', fromEditor: true, stagedPath })
		deployLabel = label
		return new Promise((resolve) => (answerDeploy = resolve))
	}

	function settleDeploy(deploy: boolean) {
		const answer = answerDeploy
		answerDeploy = undefined
		answer?.(deploy)
	}

	function close(deploy = false) {
		settleDeploy(deploy)
		open = false
	}

	async function loadApp() {
		const asked = path
		const row = await AppService.getAppLiteByPath({ workspace: $operatingWorkspace!, path })
		if (path === asked) app = row
	}

	async function loadRunnableIdentity() {
		const asked = path
		const workspace = $operatingWorkspace!
		const row =
			kind === 'flow'
				? await FlowService.getFlowByPath({ workspace, path })
				: await ScriptService.getScriptByPath({ workspace, path })
		if (path !== asked) return
		runnableIdentity = {
			runAs: row.on_behalf_of ?? row.on_behalf_of_email ?? undefined,
			canEdit: editRights.canEditItem(kind, path, row.extra_perms)
		}
	}

	async function readFolder(p: string): Promise<Folder | undefined> {
		const folder = folderOfPath(p)
		if (folder === undefined) return undefined
		// A user outside the folder cannot read it; the loss is then not listed.
		return FolderService.getFolder({ workspace: $operatingWorkspace!, name: folder }).catch(
			() => undefined
		)
	}

	async function loadMoveLoss() {
		const read = ++moveLossReads
		const from = path
		const to = moveTo
		if (!to) return
		try {
			const [fromFolder, toFolder, members] = await Promise.all([
				readFolder(from),
				readFolder(to),
				GranularAclService.getGranularAcls({
					workspace: $operatingWorkspace!,
					path: from,
					kind: kind as any
				})
			])
			const after = inheritedAccess(to, toFolder)
			const groups = new Set(
				[...(after?.rows ?? []).map((r) => r.name), ...Object.keys(members)].filter((n) =>
					n.startsWith('g/')
				)
			)
			const groupMembers: Record<string, string[]> = Object.fromEntries(
				await Promise.all(
					[...groups].map(
						async (g): Promise<[string, string[]]> => [
							g,
							await GroupService.getGroup({ workspace: $operatingWorkspace!, name: g.slice(2) })
								.then((group) => group.members ?? [])
								.catch(() => [])
						]
					)
				)
			)
			if (read !== moveLossReads) return
			moveLoss = accessLostByMove(inheritedAccess(from, fromFolder), after, members, groupMembers)
		} catch {
			if (read === moveLossReads) moveLoss = undefined
		}
	}

	function names(list: string[]): string {
		if (list.length <= 3) return list.join(', ').replace(/, ([^,]*)$/, ' and $1')
		return `${list.slice(0, 3).join(', ')} and ${list.length - 3} more`
	}

	let moveWarning = $derived.by(() => {
		if (!moveTo) return undefined
		const from = ownerLabel(path)
		const gains = folderOfPath(moveTo) !== undefined || moveTo.startsWith('g/')
		const parts: string[] = []
		if (!moveLoss) {
			parts.push(`Members of ${from} lose the access they have through it.`)
		} else if (moveLoss.lost.length || moveLoss.demoted.length) {
			const lost = moveLoss.lost.length
				? `${names(moveLoss.lost)} ${moveLoss.lost.length === 1 ? 'loses' : 'lose'} the access ${moveLoss.lost.length === 1 ? 'it has' : 'they have'} through ${from}`
				: undefined
			const demoted = moveLoss.demoted.length
				? `${names(moveLoss.demoted)} can then only view it`
				: undefined
			parts.push(`${[lost, demoted].filter(Boolean).join(', and ')}. Add them here to keep it.`)
		} else {
			parts.push(`Everyone with access through ${from} keeps it.`)
		}
		if (gains) parts.push(`Members of ${ownerLabel(moveTo)} gain access.`)
		return {
			title: `Deploying moves this ${noun} from ${from} to ${ownerLabel(moveTo)}`,
			body: parts.join(' ')
		}
	})

	const ACCESS_MODE_LABELS: Record<AppAccessMode, string> = {
		publisher: 'Members',
		guest: 'Guests',
		anonymous: 'Public'
	}

	async function changeApp(
		change: { execution_mode: AppAccessMode } | { custom_path: string | null }
	) {
		appBusy = true
		try {
			app = await updateDeployedAppAccess($operatingWorkspace!, path, change)
			onAppChanged?.(app)
			onChange?.()
			sendUserToast(
				'execution_mode' in change
					? `Who can open ${name} changed to ${ACCESS_MODE_LABELS[change.execution_mode]}`
					: change.custom_path
						? `Custom URL of ${name} set to ${change.custom_path}`
						: `Custom URL of ${name} removed`
			)
		} catch (err) {
			sendUserToast(`Could not change the app's access: ${err?.body ?? err}`, true)
			// Re-read so the controls show what is deployed, not the choice that was refused.
			await loadApp()
		} finally {
			appBusy = false
		}
	}
</script>

<!-- Closed by Escape, the backdrop, the close button or Cancel: a deploy waiting on it does not go
     on. -->
<Modal
	bind:open={() => open, (v) => (v ? (open = true) : close())}
	title="Share {name}"
	kind="X"
	enterConfirms={false}
	fitHeight
	class="sm:max-w-xl"
>
	<!-- A long access list gives up height when the sections below grow, and keeps the dialog
	     inside the viewport. -->
	<div class="flex flex-col gap-6 h-full">
		{#if moveWarning}
			<Alert type="warning" size="xs" title={moveWarning.title}>
				{moveWarning.body} Changes here apply to the deployed {noun} right away.
			</Alert>
		{:else if fromEditor}
			<!-- From an editor, where every other change waits for Deploy, say which version this is. -->
			<Alert type="info" size="xs" title="Changes here apply to the deployed {noun} right away" />
		{/if}

		{#key path}
			<ItemAccessList
				{kind}
				{path}
				onChange={() => {
					onChange?.()
					if (moveTo) void loadMoveLoss()
				}}
			/>
		{/key}

		{#if isApp}
			<section class="flex flex-col gap-2">
				<h3 class="text-sm font-semibold text-emphasis">Who can open the app</h3>
				{#if app}
					{#if !canWriteApp}
						<span class="text-xs text-secondary">
							Only people who can edit this app can change who can open it.
						</span>
					{:else if modeRefusal}
						<span class="text-xs text-secondary">
							{#if modeRefusal === 'unsandboxed'}
								This app is not sandboxed yet. Deploy it so it is sandboxed, then change who can
								open it.
							{:else}
								This app runs on behalf of {runAs}. Deploy it so it runs on behalf of you, then
								change who can open it.
							{/if}
						</span>
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
							disabled={appBusy || !canWriteApp || !!modeRefusal}
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

		{#if permissions}
			<Section label="Permissions" collapsable>
				{#snippet badge()}
					<span class="text-xs text-secondary">{permissions.summary}</span>
				{/snippet}
				<div class="flex flex-col gap-1 text-xs text-secondary">
					<span class="font-medium text-emphasis">{permissions.summary}</span>
					<span>{permissions.body}</span>
					{#if permissions.canEdit}
						<span>
							{fromEditor
								? 'To change it, edit Settings › Permissions and deploy.'
								: `To change it, edit Settings › Permissions in the ${noun}'s editor and deploy.`}
						</span>
					{/if}
				</div>
			</Section>
		{/if}

		<div class="flex flex-row items-center gap-2">
			<span class="grow text-xs text-secondary">
				{fromEditor ? '' : 'Changes apply right away.'}
			</span>
			{#if deployLabel}
				<Button variant="default" unifiedSize="md" onclick={() => close()}>Cancel</Button>
				<Button variant="accent" unifiedSize="md" onclick={() => close(true)}>
					{deployLabel}
				</Button>
			{:else}
				<Button variant="accent" unifiedSize="md" onclick={() => close()}>Done</Button>
			{/if}
		</div>
	</div>
</Modal>
