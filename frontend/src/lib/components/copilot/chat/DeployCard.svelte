<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'

	// It reaches the session tool graph, which the transcript must not load to render a card.
	const deployDiffModule = lazyModule(() => import('./global/deployDiff'))
</script>

<script lang="ts">
	import { resource } from 'runed'
	import { FolderService } from '$lib/gen'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import { folderOfPath, inheritedAccess } from '$lib/components/itemSettings/inheritedAccess'
	import DeployCardView, { type DeployCardReview } from './DeployCardView.svelte'
	import { deployIdentity, newAppAccessMode } from './deployCardFacts'
	import { getChatViewHost } from './chatViewHost'
	import type { ToolDisplayMessage } from './shared'
	import type { TriggerKind, WorkspaceItemType } from './global/workspaceItems'

	interface Props {
		/** A `deploy_workspace_item` call waiting on the user. */
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const chatHost = getChatViewHost()
	const workspace = $derived(chatHost.operatingWorkspace)
	const args = $derived(
		(message.parameters ?? {}) as {
			type?: WorkspaceItemType
			path?: string
			trigger_kind?: TriggerKind
			deployment_message?: string
			force?: boolean
		}
	)

	async function readDiff(target: {
		workspace: string | undefined
		type?: WorkspaceItemType
		path?: string
		trigger_kind?: TriggerKind
	}) {
		if (!target.workspace) return undefined
		const { loadDeployDiff } = await deployDiffModule.load()
		return loadDeployDiff(target.workspace, target)
	}

	const review = resource(
		() => ({ workspace, type: args.type, path: args.path, trigger_kind: args.trigger_kind }),
		readDiff
	)
	const reviewState: DeployCardReview = $derived(
		review.current
			? { state: 'ready', ...review.current }
			: review.error
				? { state: 'error', message: review.error.message }
				: { state: 'loading' }
	)

	// Where the deploy lands: the draft's path unless the user picks another for a first deploy.
	let deployPath: string | undefined = $state(undefined)
	const draftPath = $derived(review.current?.path ?? args.path)
	const targetPath = $derived(deployPath ?? draftPath)
	const pathKind = $derived(
		review.current?.kind === 'script' || review.current?.kind === 'flow'
			? review.current.kind
			: review.current?.kind === 'raw_app' || review.current?.kind === 'app'
				? 'app'
				: undefined
	)

	const user = useActingUser(() => workspace)
	const isNew = $derived(review.current?.isNew === true)
	const folderDefault = useFolderDefaultPermissionedAs(
		() => (isNew ? targetPath : undefined),
		() => workspace
	)
	const me = $derived(user.current?.username ? `u/${user.current.username}` : undefined)

	const identity = $derived(
		review.current && !folderDefault.loading
			? deployIdentity(review.current.kind, {
					deployed: review.current.deployed,
					draft: review.current.draft,
					isNew,
					folderDefault: folderDefault.value
				})
			: undefined
	)

	// Who reaches a new item through its path. An update keeps whoever already has it.
	const access = resource(
		() => ({ workspace, path: isNew ? targetPath : undefined }),
		async ({ workspace, path }) => {
			if (!workspace || !path) return undefined
			const folderName = folderOfPath(path)
			const folder = folderName
				? await FolderService.getFolder({ workspace, name: folderName }).catch(() => undefined)
				: undefined
			return inheritedAccess(path, folder)
		}
	)

	const appAccess = $derived(
		review.current ? newAppAccessMode(review.current.kind, review.current.draft, isNew) : undefined
	)

	// Read again on click: the draft may have changed since the card first read it.
	const loadDiff = $derived.by(() => {
		const target = { workspace, type: args.type, path: args.path, trigger_kind: args.trigger_kind }
		if (!target.workspace || !target.path) return undefined
		return async () => {
			const diff = await readDiff(target)
			if (!diff) throw new Error('No workspace')
			return diff
		}
	})
</script>

<DeployCardView
	toolCallId={message.tool_call_id}
	{workspace}
	type={args.type}
	path={draftPath}
	{pathKind}
	bind:deployPath
	review={reviewState}
	{me}
	access={isNew ? access.current : undefined}
	{appAccess}
	{identity}
	deploymentMessage={args.deployment_message}
	force={args.force}
	{loadDiff}
/>
