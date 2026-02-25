<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
	import { base } from '$lib/base'
	import { enterpriseLicense, superadmin, userStore, workspaceStore } from '$lib/stores'
	import {
		AppService,
		FlowService,
		ResourceService,
		ScheduleService,
		UserService,
		WorkspaceService
	} from '$lib/gen'
	import { getAllModules } from './flows/flowExplorer'
	import Button from './common/button/Button.svelte'
	import Tooltip from './Tooltip.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { DiffIcon, Loader2 } from 'lucide-svelte'
	import Badge from './common/badge/Badge.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import {
		getTriggerDependency,
		type AdditionalInformation,
		type Kind
	} from '$lib/utils_deployable'
	import {
		checkItemExists,
		deployItem,
		getItemValue,
		getOnBehalfOfEmail
	} from '$lib/utils_workspace_deploy'
	import type { App } from './apps/types'
	import { getAllGridItems } from './apps/editor/appUtils'
	import { isRunnableByPath } from './apps/inputType'
	import type { Runnable } from './raw_apps/utils'
	import WorkspaceDeployLayout from './WorkspaceDeployLayout.svelte'
	import OnBehalfOfSelector, {
		needsOnBehalfOfSelection,
		type OnBehalfOfChoice
	} from './OnBehalfOfSelector.svelte'
	import ParentWorkspaceProtectionAlert from './ParentWorkspaceProtectionAlert.svelte'

	const dispatch = createEventDispatcher()

	interface Props {
		kind: Kind
		initialPath?: string
		additionalInformation?: AdditionalInformation | undefined
		workspaceToDeployTo?: string | undefined
		hideButton?: boolean
		canDeployToWorkspace?: boolean
	}

	let {
		kind,
		initialPath = '',
		additionalInformation = undefined,
		workspaceToDeployTo = $bindable(undefined),
		hideButton = false,
		canDeployToWorkspace = $bindable(true)
	}: Props = $props()

	let canSeeTarget: 'yes' | 'cant-deploy-to-workspace' | 'cant-see-all-deps' | undefined =
		$state(undefined)

	type Dependency = { kind: Kind; path: string; include: boolean }
	let dependencies: Dependency[] | undefined = $state<Dependency[] | undefined>(undefined)

	const allAlreadyExists: { [key: string]: boolean } = $state({})

	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let notSet: boolean | undefined = $state(undefined)
	let isFlow: boolean | undefined = $state(undefined)

	// On-behalf-of tracking for flows and scripts
	// Source workspace on_behalf_of emails (keyed by kind:path)
	let sourceOnBehalfOfInfo = $state<Record<string, string | undefined>>({})
	// Target workspace on_behalf_of emails (keyed by kind:path)
	let targetOnBehalfOfInfo = $state<Record<string, string | undefined>>({})
	let onBehalfOfChoice = $state<Record<string, OnBehalfOfChoice>>({})
	let canPreserveOnBehalfOf = $state(false)

	// Check if an item needs on_behalf_of selection (more than 1 unique option)
	function itemNeedsOnBehalfOfSelection(statusPath: string, kind: string): boolean {
		const myIdentity = kind === 'trigger' ? $userStore?.username : $userStore?.email
		return needsOnBehalfOfSelection(
			kind,
			sourceOnBehalfOfInfo[statusPath],
			targetOnBehalfOfInfo[statusPath],
			myIdentity
		)
	}

	// Get the email to use for deployment based on user's choice
	function getOnBehalfOfEmailForDeploy(statusPath: string): string | undefined {
		const choice = onBehalfOfChoice[statusPath]
		if (choice === 'source') return sourceOnBehalfOfInfo[statusPath]
		if (choice === 'target') return targetOnBehalfOfInfo[statusPath]
		// 'me' or undefined = don't pass, backend will use deploying user's email
		return undefined
	}

	async function reload(path: string) {
		try {
			if (!$superadmin) {
				const targetUser = await UserService.whoami({ workspace: workspaceToDeployTo! })
				canPreserveOnBehalfOf =
					targetUser.is_admin ||
					targetUser.groups?.includes('wm_deployers') ||
					false
			} else {
				canPreserveOnBehalfOf = true
			}
			canSeeTarget = 'yes'
		} catch {
			canSeeTarget = 'cant-deploy-to-workspace'
			canPreserveOnBehalfOf = false
			return
		}

		let allDeps
		try {
			allDeps = await getDependencies(kind, path)
		} catch {
			canSeeTarget = 'cant-see-all-deps'
			return
		}

		let sortedSet: { kind: Kind; path: string }[] = []
		allDeps.forEach((x) => {
			if (!sortedSet.find((y) => y.kind == x.kind && y.path == x.path)) {
				sortedSet.push(x)
			}
		})
		for (const dep of sortedSet) {
			allAlreadyExists[computeStatusPath(dep.kind, dep.path)] = await checkAlreadyExists(
				dep.kind,
				dep.path
			)
		}
		dependencies = sortedSet.map((x) => ({
			...x,
			include:
				x.kind != 'variable' &&
				x.kind != 'resource' &&
				x.kind != 'resource_type' &&
				(x.kind != 'folder' || !allAlreadyExists[computeStatusPath(x.kind, x.path)])
		}))

		// Fetch on_behalf_of_email for flows, scripts, apps, and triggers from both workspaces
		for (const dep of sortedSet.filter((d) =>
			['flow', 'script', 'app', 'trigger'].includes(d.kind)
		)) {
			const key = computeStatusPath(dep.kind, dep.path)
			try {
				sourceOnBehalfOfInfo[key] = await getOnBehalfOfEmail(
					dep.kind,
					dep.path,
					$workspaceStore!,
					additionalInformation
				)
			} catch {
				sourceOnBehalfOfInfo[key] = undefined
			}
			try {
				targetOnBehalfOfInfo[key] = await getOnBehalfOfEmail(
					dep.kind,
					dep.path,
					workspaceToDeployTo!,
					additionalInformation
				)
			} catch {
				targetOnBehalfOfInfo[key] = undefined
			}
		}
	}

	async function getDependencies(
		kind: Kind,
		path: string
	): Promise<{ kind: Kind; path: string }[]> {
		async function rec(kind: Kind, path: string): Promise<{ kind: Kind; path: string }[]> {
			if (kind == 'schedule') {
				const schedule = await ScheduleService.getSchedule({ workspace: $workspaceStore!, path })
				if (schedule.script_path && schedule.script_path != '') {
					if (schedule.script_path) {
						return [{ kind: 'script', path: schedule.script_path }]
					} else {
						return [{ kind: 'flow', path: schedule.script_path }]
					}
				} else {
					return []
				}
			} else if (kind == 'flow') {
				const flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
				return getAllModules(flow.value.modules, flow.value.failure_module).flatMap((x) => {
					let result: { kind: Kind; path: string }[] = []
					if (x.value.type == 'script' || x.value.type == 'rawscript' || x.value.type == 'flow') {
						Object.values(x.value.input_transforms).forEach((y) => {
							if (y.type == 'static' && typeof y.value == 'string') {
								if (y.value.startsWith('$res:')) {
									result.push({ kind: 'resource', path: y.value.substring(5) })
								} else if (y.value.startsWith('$var:')) {
									result.push({ kind: 'variable', path: y.value.substring(5) })
								}
							}
						})
					}
					if (x.value.type == 'script') {
						if (x.value.path && !x.value.path.startsWith('hub/')) {
							result.push({ kind: 'script', path: x.value.path })
						}
					} else if (x.value.type == 'flow') {
						if (x.value.path) {
							result.push({ kind: 'flow', path: x.value.path })
						}
					}
					return result
				})
			} else if (kind == 'app') {
				const app = await AppService.getAppByPath({ workspace: $workspaceStore!, path })
				let result: { kind: Kind; path: string }[] = []
				if (app.raw_app) {
					const rawAppValue = app.value as { runnables?: Record<string, Runnable> }
					for (const runnable of Object.values(rawAppValue.runnables ?? {})) {
						if (isRunnableByPath(runnable)) {
							if (runnable.runType == 'script') {
								result.push({ kind: 'script', path: runnable.path })
							} else if (runnable.runType == 'flow') {
								result.push({ kind: 'flow', path: runnable.path })
							}
						}
					}
				} else {
					let appValue = app.value as App
					getAllGridItems(appValue).forEach((gridItem) => {
						const ci = gridItem.data.componentInput
						if (ci?.type == 'runnable' && isRunnableByPath(ci.runnable)) {
							if (ci.runnable.runType == 'script') {
								result.push({ kind: 'script', path: ci.runnable.path })
							} else if (ci.runnable.runType == 'flow') {
								result.push({ kind: 'flow', path: ci.runnable.path })
							}
						}
					})
				}
				return result
			} else if (kind == 'resource') {
				const res = await ResourceService.getResource({ workspace: $workspaceStore!, path })
				function recObj(obj: any) {
					if (typeof obj == 'string' && obj.startsWith('$var:')) {
						return [{ kind: 'variable', path: obj.substring(5) }]
					} else if (typeof obj == 'object') {
						return Object.values(obj).flatMap((x) => recObj(x))
					} else {
						return []
					}
				}

				return [...recObj(res.value), { kind: 'resource_type', path: res.resource_type }]
			} else if (kind == 'trigger') {
				if (additionalInformation?.triggers) {
					return getTriggerDependency(additionalInformation.triggers.kind, path, $workspaceStore!)
				}
				throw new Error('Missing trigger information')
			}
			return []
		}
		let toProcess = [{ kind, path }]
		let processed: { kind: Kind; path: string }[] = []
		while (toProcess.length > 0) {
			const { kind, path } = toProcess.pop()!
			toProcess.push(...(await rec(kind, path)))
			processed.push({ kind, path })
		}
		let folders: string[] = []
		for (const p of processed) {
			let split = p.path.split('/')
			if (split.length > 2 && split[0] == 'f' && !folders.includes(split[1])) {
				folders.push(split[1])
				processed.push({ kind: 'folder', path: split[1] })
			}
		}
		return processed
	}

	async function checkAlreadyExists(kind: Kind, path: string): Promise<boolean> {
		return checkItemExists(kind, path, workspaceToDeployTo!, additionalInformation)
	}

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = $state({})

	async function deploy(kind: Kind, path: string) {
		const statusPath = computeStatusPath(kind, path)
		deploymentStatus[statusPath] = { status: 'loading' }

		const result = await deployItem({
			kind,
			path,
			workspaceFrom: $workspaceStore!,
			workspaceTo: workspaceToDeployTo!,
			additionalInformation,
			onBehalfOfEmail: getOnBehalfOfEmailForDeploy(statusPath)
		})

		if (result.success) {
			allAlreadyExists[statusPath] = true
			deploymentStatus[statusPath] = { status: 'deployed' }
		} else {
			deploymentStatus[statusPath] = { status: 'failed', error: result.error }
		}
	}

	export function deployAll() {
		dependencies?.slice().forEach(async ({ kind, path, include }) => {
			if (include) {
				await deploy(kind, path)
			}
		})
		dispatch('update', initialPath)
	}

	function computeStatusPath(kind: Kind, path: string) {
		return `${kind}:${path}`
	}

	async function showDiff(kind: Kind, path: string) {
		diffDrawer?.openDrawer()
		let values = await Promise.all([
			getItemValue(kind, path, workspaceToDeployTo!, additionalInformation),
			getItemValue(kind, path, $workspaceStore!, additionalInformation)
		])
		diffDrawer?.setDiff({
			mode: 'simple',
			original: values?.[0] as any,
			current: values?.[1] as any,
			title: 'Staging/prod <> Dev'
		})
	}

	$effect(() => {
		WorkspaceService.getDeployTo({ workspace: $workspaceStore! }).then((x) => {
			workspaceToDeployTo = x.deploy_to
			if (x.deploy_to == undefined) {
				notSet = true
			}
		})
	})

	$effect(() => {
		workspaceToDeployTo && initialPath && untrack(() => reload(initialPath))
	})

	// Transform dependencies to deployable item format for the shared layout
	let deployableItems = $derived(
		(dependencies ?? []).map((dep) => ({
			key: computeStatusPath(dep.kind, dep.path),
			path: dep.path,
			kind: dep.kind,
			include: dep.include,
			triggerKind: dep.kind === 'trigger' ? additionalInformation?.triggers?.kind : undefined
		}))
	)

	let selectedItems = $derived<string[]>(
		(dependencies ?? [])
			.filter((dep) => dep.include)
			.map((dep) => computeStatusPath(dep.kind, dep.path))
	)

	let allSelected = $derived(
		dependencies != null && dependencies.length > 0 && dependencies.every((dep) => dep.include)
	)

	// Check if all required on_behalf_of selections are made
	let hasUnselectedOnBehalfOf = $derived(
		selectedItems.some((statusPath) => {
			const dep = dependencies?.find((d) => computeStatusPath(d.kind, d.path) === statusPath)
			if (!dep) return false
			return (
				itemNeedsOnBehalfOfSelection(statusPath, dep.kind) &&
				onBehalfOfChoice[statusPath] === undefined
			)
		})
	)

	function toggleItem(item: { key: string }) {
		if (dependencies) {
			const idx = dependencies.findIndex(
				(dep) => computeStatusPath(dep.kind, dep.path) === item.key
			)
			if (idx !== -1) {
				dependencies[idx].include = !dependencies[idx].include
			}
		}
	}

	function selectAll() {
		if (dependencies) {
			dependencies = dependencies.map((dep) => ({ ...dep, include: true }))
		}
	}

	function deselectAll() {
		if (dependencies) {
			dependencies = dependencies.map((dep) => ({ ...dep, include: false }))
		}
	}
</script>

<div class="mt-6"></div>

{#if !$enterpriseLicense}
	<Alert type="warning" title="Enterprise license required"
		>Deploy to staging/prod from the web UI is only available with an enterprise license</Alert
	>
{:else if notSet == true}
	<Alert type="error" title="Staging/Prod deploy not set up"
		>As an admin, go to Settings {'->'} Workspace {'->'} Deployment UI</Alert
	>
{:else}
	<Alert type="info" title="Shareable page"
		>Share this <a href="{base}/deploy/{kind}/{initialPath}">link</a> to have another properly permissioned
		user do the deployment</Alert
	>

	<h3 class="mb-2 mt-8"
		>Destination Workspace&nbsp; <Tooltip
			>Workspace to deploy to is set in the workspace settings</Tooltip
		></h3
	>
	<input class="max-w-xs" type="text" disabled value={workspaceToDeployTo} />

	{#if workspaceToDeployTo}
		<ParentWorkspaceProtectionAlert
			parentWorkspaceId={workspaceToDeployTo}
			onUpdateCanDeploy={(canDeploy) => {
				canDeployToWorkspace = canDeploy
			}}
		/>
	{/if}

	{#if canSeeTarget == undefined}
		<div class="mt-6"></div>
		<Loader2 class="animate-spin" />
	{:else if canSeeTarget == 'yes'}
		<h3 class="mb-6 mt-16">All related deployable items</h3>

		<DiffDrawer bind:this={diffDrawer} {isFlow} />

		<WorkspaceDeployLayout
			items={deployableItems}
			{selectedItems}
			{deploymentStatus}
			selectablePredicate={() => true}
			{allSelected}
			onToggleItem={toggleItem}
			onSelectAll={selectAll}
			onDeselectAll={deselectAll}
			emptyMessage="No deployable items found"
		>
			{#snippet itemActions(item)}
				{@const statusPath = item.key}
				{@const exists = allAlreadyExists[statusPath]}
				{@const status = deploymentStatus[statusPath]}
				{@const sourceEmail = sourceOnBehalfOfInfo[statusPath]}
				{@const targetEmail = targetOnBehalfOfInfo[statusPath]}

				<!-- On-behalf-of selector -->
				{#if itemNeedsOnBehalfOfSelection(statusPath, item.kind)}
					<OnBehalfOfSelector
						{sourceEmail}
						{targetEmail}
						selected={onBehalfOfChoice[statusPath]}
						onSelect={(choice) => (onBehalfOfChoice[statusPath] = choice)}
						kind={item.kind}
						canPreserve={canPreserveOnBehalfOf}
					/>
				{/if}

				{#if exists === false}
					{#if item.include}
						<Badge
							>New <Tooltip
								>This {item.kind} doesn't exist yet on the target and will be created by the deployment</Tooltip
							></Badge
						>
					{:else}
						<Badge color="red">
							Missing
							<Tooltip
								>{#if item.kind == 'resource_type'}
									Resource types are not re-deployed by default. We strongly recommend to add shared
									resource types in 'admin' workspace, which will have them be shared to every
									workspace.
								{:else}
									This {item.kind} doesn't exist and is not included in the deployment. Variables and
									Resources are considered to be workspace specific and are never included by default.
								{/if}</Tooltip
							>
						</Badge>
					{/if}
				{:else if exists === true && !status}
					<Button
						size="xs"
						variant="subtle"
						onclick={() => {
							showDiff(item.kind, item.path)
							isFlow = item.kind === 'flow'
						}}
					>
						<DiffIcon class="w-3 h-3" />
						Show diff
					</Button>
				{/if}

				{#if !status}
					<Button
						color="light"
						size="xs"
						disabled={!canDeployToWorkspace ||
							(itemNeedsOnBehalfOfSelection(statusPath, item.kind) &&
								onBehalfOfChoice[statusPath] === undefined)}
						onclick={() => deploy(item.kind, item.path)}>Deploy</Button
					>
				{/if}
			{/snippet}

			{#snippet footer()}
				<div class="flex flex-col items-end gap-2">
					{#if !hideButton}
						<Button on:click={deployAll} disabled={!canDeployToWorkspace || hasUnselectedOnBehalfOf}
							>Deploy all toggled</Button
						>
					{/if}
					{#if hasUnselectedOnBehalfOf}
						<span class="text-xs text-yellow-600">
							{#if kind === 'trigger'}
								You must set the "edited by" user for all triggers before deploying
								<Tooltip class="text-yellow-600">
									The "edited by" field defines which user's permissions will be applied
									when the trigger runs. Make sure this is set to an appropriate user
									before deploying.
								</Tooltip>
							{:else}
								You must set the "on behalf of" user for all items before deploying
								<Tooltip class="text-yellow-600">
									The "run on behalf of" field defines which user's permissions will be
									applied during execution. Make sure this is set to an appropriate user
									before deploying.
								</Tooltip>
							{/if}
						</span>
					{/if}
				</div>
			{/snippet}
		</WorkspaceDeployLayout>
	{:else if canSeeTarget == 'cant-see-all-deps'}
		<div class="my-2"></div>
		<Alert type="error" title="User doesn't have visibility over all dependencies"
			>You do not have visibility over some of the dependencies of this item. Ask a permissioned
			user to deploy this item using the shareable link or get the proper permissions on the
			dependencies</Alert
		>
	{:else}
		<div class="my-2"></div>
		<Alert type="error" title="User not allowed to deploy to this workspace"
			>Ask a permissioned user to deploy this item using the shareable link or get the proper
			permissions on the target workspace</Alert
		>
	{/if}
{/if}
