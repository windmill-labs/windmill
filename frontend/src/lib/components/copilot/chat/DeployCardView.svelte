<script lang="ts" module>
	import type { OnBehalfOf } from './global/deployEdits'

	export type DeployCardReview =
		| { state: 'loading' }
		| { state: 'error'; message: string }
		| { state: 'ready'; isNew: boolean; added: number; removed: number }

	/** The run-as user of a script, flow or app being deployed. */
	export type DeployCardIdentity = {
		kind: 'script' | 'flow' | 'app'
		/** The draft's user, which the deploy keeps unless the card picks another. */
		held: OnBehalfOf
		/** The deployed user; both unset for a new item or one that runs as its caller. */
		saved: OnBehalfOf
		/** The folder default a new item with no user takes. The deploy applies it, so the setting
		 *  shows as required, as in the editors' deploy dialog. */
		folderDefault: string | undefined
		rawApp: boolean
	}
</script>

<script lang="ts">
	import { ChevronRight, Loader2 } from 'lucide-svelte'
	import { Alert, Badge, Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import type { ComponentProps } from 'svelte'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip } from '$lib/components/itemSettings/onBehalfOf'
	import AppAccessSection from '$lib/components/itemSettings/AppAccessSection.svelte'
	import type { AppAccessMode } from '$lib/components/itemSettings/appAccess'
	import { folderOfPath, type InheritedAccess } from '$lib/components/itemSettings/inheritedAccess'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolConfirmationFooter from './ToolConfirmationFooter.svelte'
	import DeployDiffButton, { type DeployDiffSides } from './DeployDiffButton.svelte'
	import { TOOL_CONFIRM_ACTIONS } from './toolConfirmAction'
	import { setDeployEdits, type DeployEdits } from './global/deployEdits'

	interface Props {
		/** Undefined outside a chat: the buttons then answer nothing. */
		toolCallId: string | undefined
		workspace: string | undefined
		type: string | undefined
		/** The draft's path, where the model asked to deploy. */
		path: string | undefined
		/** For a script, flow or app, the path a first deploy may move to; undefined keeps it fixed. */
		pathKind: ComponentProps<typeof Path>['kind'] | undefined
		/** The path the user picked; unset deploys at `path`. */
		deployPath?: string
		review: DeployCardReview
		/** `u/name` of the deploying user. */
		me: string | undefined
		/** Who reaches a new item through the path it deploys at; undefined for an update. */
		access: InheritedAccess | undefined
		/** Who can open a new app as the draft sets it; undefined for anything else. */
		appAccess: AppAccessMode | undefined
		/** Undefined for a kind that runs as its caller, or until the draft is read. */
		identity: DeployCardIdentity | undefined
		deploymentMessage: string | undefined
		force: boolean | undefined
		/** Undefined disables the Diff button. */
		loadDiff: (() => Promise<DeployDiffSides>) | undefined
	}

	let {
		toolCallId,
		workspace,
		type,
		path,
		pathKind,
		deployPath = $bindable(),
		review,
		me,
		access,
		appAccess,
		identity,
		deploymentMessage,
		force,
		loadDiff
	}: Props = $props()

	const confirm = TOOL_CONFIRM_ACTIONS.deploy
	const isNew = $derived(review.state === 'ready' && review.isNew)
	const kindLabel = $derived(type ?? 'item')
	const targetPath = $derived(deployPath ?? path)
	let pathError: string | undefined = $state(undefined)

	// What the user typed or picked; unset keeps what the model asked for or the draft holds.
	let message: string | undefined = $state(undefined)
	let picked: OnBehalfOf | undefined = $state(undefined)
	let pickedAccess: AppAccessMode | undefined = $state(undefined)
	const runAs: OnBehalfOf | undefined = $derived(picked ?? identity?.held)

	function edits(): DeployEdits {
		const held = identity?.held
		const changedUser =
			picked !== undefined &&
			(picked.email !== held?.email || picked.permissionedAs !== held?.permissionedAs)
		return {
			deploymentMessage: message,
			deployPath: isNew && deployPath && deployPath !== path ? deployPath : undefined,
			onBehalfOf: changedUser ? picked : undefined,
			appAccess: pickedAccess !== undefined && pickedAccess !== appAccess ? pickedAccess : undefined
		}
	}

	const accessSummary = $derived.by(() => {
		if (!access || !targetPath || pathError) return undefined
		const folderName = folderOfPath(targetPath)
		if (folderName !== undefined) {
			if (access.note !== undefined) return `Members of folder ${folderName}`
			const n = access.rows.length
			return `Folder ${folderName} · ${n} ${n === 1 ? 'member' : 'members'}`
		}
		const owner = access.rows[0]?.name
		return owner === me ? 'Only you' : owner
	})
	let accessOpen = $state(false)
</script>

{#snippet headerRight()}
	{#if review.state === 'ready'}
		{#if review.isNew}
			<Badge small color="blue">New</Badge>
		{:else if review.added || review.removed}
			<span class="font-mono text-2xs shrink-0">
				<span class="text-green-600 dark:text-green-400">+{review.added}</span>
				<span class="text-red-600 dark:text-red-400">−{review.removed}</span>
			</span>
		{/if}
	{/if}
{/snippet}

<ChatCollapsibleCard
	label="Deploy {kindLabel}"
	expanded
	onToggle={() => {}}
	toggleable={false}
	labelClass="truncate"
	contentClass="space-y-4"
	{headerRight}
>
	<div class="flex flex-col gap-4 font-main">
		{#if review.state === 'loading'}
			<span class="flex items-center gap-2 text-xs text-secondary">
				<Loader2 size={12} class="animate-spin" /> Reading the draft…
			</span>
		{:else if review.state === 'error'}
			<Alert type="error" size="xs" title="Could not read the draft">{review.message}</Alert>
		{/if}

		{#if isNew && pathKind && workspace}
			<Label label="Path">
				<Path
					size="sm"
					kind={pathKind}
					initialPath={path ?? ''}
					bind:path={() => targetPath, (v) => (deployPath = v)}
					bind:error={pathError}
					workspaceOverride={workspace}
					autofocus={false}
				/>
			</Label>
		{:else if path}
			<Label label="Path">
				<span class="text-xs font-mono text-primary truncate" title={path}>{path}</span>
			</Label>
		{/if}

		{#if force}
			<Alert type="warning" size="xs" title="Overwrites a newer deploy">
				Someone deployed this {kindLabel} after the draft was started. Deploying replaces their version.
			</Alert>
		{/if}

		{#if isNew && accessSummary}
			<Label label="Who gets access">
				<div>
					<Button
						variant="default"
						unifiedSize="sm"
						endIcon={{ icon: ChevronRight }}
						wrapperClasses="w-fit max-w-full"
						title="See who reaches this {kindLabel} once it is deployed"
						onClick={() => (accessOpen = true)}
					>
						<span class="truncate">{accessSummary}</span>
					</Button>
				</div>
			</Label>
		{/if}

		{#if appAccess && workspace && targetPath}
			<Label label="Who can open it">
				<AppAccessSection
					{workspace}
					appPath={targetPath}
					rawApp={identity?.rawApp ?? true}
					mode={pickedAccess ?? appAccess}
					onModeChange={(m) => {
						pickedAccess = m
					}}
				/>
			</Label>
		{/if}

		{#if identity && workspace}
			<RunOnBehalfOfSetting
				kind={identity.kind}
				required={identity.kind === 'app' || !!identity.folderDefault}
				tooltip={identity.kind === 'app' ? appOnBehalfOfTooltip(identity.rawApp) : undefined}
				bind:email={
					() => runAs?.email, (v) => (picked = { email: v, permissionedAs: runAs?.permissionedAs })
				}
				bind:permissionedAs={
					() => runAs?.permissionedAs, (v) => (picked = { email: runAs?.email, permissionedAs: v })
				}
				savedEmail={identity.saved.email}
				savedPermissionedAs={identity.saved.permissionedAs}
				targetWorkspace={workspace}
				{isNew}
				folderDefault={identity.folderDefault}
				hideDescription
			/>
		{/if}

		<Label label="Deployment message">
			<TextInput
				size="sm"
				bind:value={() => message ?? deploymentMessage ?? '', (v) => (message = String(v ?? ''))}
				inputProps={{ placeholder: 'Optional' }}
			/>
		</Label>
	</div>

	<ToolConfirmationFooter
		{toolCallId}
		rejectLabel="Don't deploy"
		confirmLabel={confirm.label}
		confirmIcon={confirm.icon}
		confirmDisabled={!!pathError}
		onConfirm={() => toolCallId && setDeployEdits(toolCallId, edits())}
	>
		{#snippet leading()}
			<DeployDiffButton load={loadDiff} />
		{/snippet}
	</ToolConfirmationFooter>
</ChatCollapsibleCard>

{#if access}
	<Modal bind:open={accessOpen} title="Who gets access" kind="X" class="sm:max-w-md">
		<div class="flex flex-col gap-3 text-xs">
			<p class="text-secondary">
				Once deployed, {targetPath} is reachable through its path by:
			</p>
			{#if access.note !== undefined}
				<p class="text-primary">{access.note}</p>
			{:else}
				<ul
					class="flex flex-col divide-y divide-border-light border border-border-light rounded-md"
				>
					{#each access.rows as accessRow (accessRow.name)}
						<li class="flex items-center justify-between px-3 py-1.5">
							<span class="text-primary">
								{accessRow.name}{accessRow.name === me ? ' (you)' : ''}
							</span>
							<span class="text-secondary">{accessRow.write ? 'Can edit' : 'Can view'}</span>
						</li>
					{/each}
				</ul>
			{/if}
			<p class="text-secondary">
				Workspace admins reach everything. To share it with more people, use Share once it is
				deployed.
			</p>
		</div>
	</Modal>
{/if}
