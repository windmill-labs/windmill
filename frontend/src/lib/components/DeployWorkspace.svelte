<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { enterpriseLicense, superadmin, workspaceStore } from '$lib/stores'
	import {
		AppService,
		FlowService,
		FolderService,
		RawAppService,
		ResourceService,
		ScheduleService,
		ScriptService,
		UserService,
		VariableService,
		WorkspaceService
	} from '$lib/gen'
	import { getAllModules } from './flows/flowExplorer'
	import Button from './common/button/Button.svelte'
	import Tooltip from './Tooltip.svelte'
	import Alert from './common/alert/Alert.svelte'
	import Toggle from './Toggle.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Badge from './common/badge/Badge.svelte'
	import DiffDrawer from './DiffDrawer.svelte'

	const dispatch = createEventDispatcher()

	type Kind =
		| 'script'
		| 'resource'
		| 'schedule'
		| 'variable'
		| 'flow'
		| 'app'
		| 'raw_app'
		| 'resource_type'
		| 'folder'

	export let kind: Kind
	export let initialPath: string = ''
	export let workspaceToDeployTo: string | undefined = undefined
	export let hideButton: boolean = false

	let seeTarget: boolean | undefined = undefined

	let dependencies: { kind: Kind; path: string; include: boolean }[] | undefined = undefined

	const allAlreadyExists: { [key: string]: boolean } = {}

	let diffDrawer: DiffDrawer
	let notSet: boolean | undefined = undefined

	$: WorkspaceService.getDeployTo({ workspace: $workspaceStore! }).then((x) => {
		workspaceToDeployTo = x.deploy_to
		if (x.deploy_to == undefined) {
			notSet = true
		}
	})
	$: workspaceToDeployTo && reload(initialPath)

	async function reload(path: string) {
		try {
			if (!$superadmin) {
				await UserService.whoami({ workspace: workspaceToDeployTo! })
			}
			seeTarget = true
		} catch {
			seeTarget = false
		}

		const allDeps = await getDependencies(kind, path)
		for (const dep of allDeps) {
			allAlreadyExists[computeStatusPath(dep.kind, dep.path)] = await checkAlreadyExists(
				dep.kind,
				dep.path
			)
		}
		dependencies = allDeps.map((x) => ({
			...x,
			include:
				x.kind != 'variable' &&
				x.kind != 'resource' &&
				x.kind != 'resource_type' &&
				(x.kind != 'folder' || !allAlreadyExists[computeStatusPath(x.kind, x.path)])
		}))
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
		processed.reverse()
		return processed
	}

	async function checkAlreadyExists(kind: Kind, path: string): Promise<boolean> {
		if (kind == 'flow') {
			return await FlowService.existsFlowByPath({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'script') {
			return await ScriptService.existsScriptByPath({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'app') {
			return await AppService.existsApp({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'raw_app') {
			return await RawAppService.existsRawApp({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'variable') {
			return await VariableService.existsVariable({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'resource') {
			return await ResourceService.existsResource({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'schedule') {
			return await ScheduleService.existsSchedule({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'resource_type') {
			return await ResourceService.existsResourceType({
				workspace: workspaceToDeployTo!,
				path: path
			})
		} else if (kind == 'folder') {
			let exists = true
			try {
				await FolderService.getFolder({
					workspace: workspaceToDeployTo!,
					name: path
				})
			} catch (e) {
				exists = false
			}
			return exists
		} else {
			throw new Error(`Unknown kind ${kind}`)
		}
	}

	const deploymentStatus: Record<
		string,
		{ status: 'loading' | 'deployed' | 'failed'; error?: string }
	> = {}

	async function deploy(kind: Kind, path: string) {
		const statusPath = `${kind}:${path}`
		deploymentStatus[statusPath] = { status: 'loading' }
		try {
			let alreadyExists = await checkAlreadyExists(kind, path)
			if (kind == 'flow') {
				const flow = await FlowService.getFlowByPath({
					workspace: $workspaceStore!,
					path: path
				})
				getAllModules(flow.value.modules).forEach((x) => {
					if (x.value.type == 'script' && x.value.hash != undefined) {
						x.value.hash = undefined
					}
				})
				if (alreadyExists) {
					await FlowService.updateFlow({
						workspace: workspaceToDeployTo!,
						path: path,
						requestBody: {
							...flow
						}
					})
				} else {
					await FlowService.createFlow({
						workspace: workspaceToDeployTo!,
						requestBody: {
							...flow
						}
					})
				}
			} else if (kind == 'script') {
				const script = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: path
				})
				await ScriptService.createScript({
					workspace: workspaceToDeployTo!,
					requestBody: {
						...script,
						lock: script.lock?.split('\n'),
						parent_hash: alreadyExists
							? (
									await ScriptService.getScriptByPath({
										workspace: workspaceToDeployTo!,
										path: path
									})
							  ).hash
							: undefined
					}
				})
			} else if (kind == 'app') {
				const app = await AppService.getAppByPath({
					workspace: $workspaceStore!,
					path: path
				})
				if (alreadyExists) {
					await AppService.updateApp({
						workspace: workspaceToDeployTo!,
						path: path,
						requestBody: {
							...app
						}
					})
				} else {
					await AppService.createApp({
						workspace: workspaceToDeployTo!,
						requestBody: {
							...app
						}
					})
				}
			} else if (kind == 'variable') {
				const variable = await VariableService.getVariable({
					workspace: $workspaceStore!,
					path: path,
					decryptSecret: true
				})
				if (alreadyExists) {
					await VariableService.updateVariable({
						workspace: workspaceToDeployTo!,
						path: path,
						requestBody: {
							path: path,
							value: variable.value ?? '',
							is_secret: variable.is_secret,
							description: variable.description ?? ''
						},
						alreadyEncrypted: false
					})
				} else {
					await VariableService.createVariable({
						workspace: workspaceToDeployTo!,
						requestBody: {
							path: path,
							value: variable.value ?? '',
							is_secret: variable.is_secret,
							description: variable.description ?? ''
						}
					})
				}
			} else if (kind == 'resource') {
				const resource = await ResourceService.getResource({
					workspace: $workspaceStore!,
					path: path
				})
				if (alreadyExists) {
					await ResourceService.updateResource({
						workspace: workspaceToDeployTo!,
						path: path,
						requestBody: {
							path: path,
							value: resource.value ?? '',
							description: resource.description ?? ''
						}
					})
				} else {
					await ResourceService.createResource({
						workspace: workspaceToDeployTo!,
						requestBody: {
							path: path,
							value: resource.value ?? '',
							resource_type: resource.resource_type,
							description: resource.description ?? ''
						}
					})
				}
			} else if (kind == 'resource_type') {
				const resource = await ResourceService.getResourceType({
					workspace: $workspaceStore!,
					path: path
				})
				if (alreadyExists) {
					await ResourceService.updateResourceType({
						workspace: workspaceToDeployTo!,
						path: path,
						requestBody: {
							schema: resource.schema,
							description: resource.description ?? ''
						}
					})
				} else {
					await ResourceService.createResourceType({
						workspace: workspaceToDeployTo!,
						requestBody: {
							description: resource.description ?? '',
							schema: resource.schema,
							name: resource.name
						}
					})
				}
			} else if (kind == 'raw_app') {
				throw new Error('Raw app deploy not implemented yet')
				// const app = await RawAppService.getRawAppData({
				// 	workspace: $workspaceStore!,
				// 	path: path
				// })
				// if (alreadyExists) {
				// }
				// await RawAppService.updateRawApp({
				// 	workspace: $workspaceStore!,
				// 	path: path,
				// 	requestBody: {
				// 		path: path
				// 	}
				// })
			} else if (kind == 'folder') {
				await FolderService.createFolder({
					workspace: workspaceToDeployTo!,
					requestBody: {
						name: path
					}
				})
			} else {
				throw new Error(`Unknown kind ${kind}`)
			}

			allAlreadyExists[statusPath] = true
			deploymentStatus[statusPath] = { status: 'deployed' }
		} catch (e) {
			deploymentStatus[statusPath] = { status: 'failed', error: e.body || e.message }
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

	async function getValue(kind: Kind, path: string, workspace: string) {
		try {
			if (kind == 'flow') {
				const flow = await FlowService.getFlowByPath({
					workspace: workspace,
					path: path
				})
				getAllModules(flow.value.modules).forEach((x) => {
					if (x.value.type == 'script' && x.value.hash != undefined) {
						x.value.hash = undefined
					}
				})
				return flow.value
			} else if (kind == 'script') {
				const script = await ScriptService.getScriptByPath({
					workspace: workspace,
					path: path
				})
				return {
					content: script.content,
					lock: script.lock,
					schema: script.schema,
					summary: script.summary,
					language: script.language
				}
			} else if (kind == 'app') {
				const app = await AppService.getAppByPath({
					workspace: workspace,
					path: path
				})
				return app
			} else if (kind == 'variable') {
				const variable = await VariableService.getVariable({
					workspace: workspace,
					path: path,
					decryptSecret: true
				})
				return variable.value
			} else if (kind == 'resource') {
				const resource = await ResourceService.getResource({
					workspace: workspace,
					path: path
				})
				return resource.value
			} else if (kind == 'resource_type') {
				const resource = await ResourceService.getResourceType({
					workspace: workspace,
					path: path
				})
				return resource.schema
			} else if (kind == 'raw_app') {
				throw new Error('Raw app deploy not implemented yet')
				// const app = await RawAppService.getRawAppData({
				// 	workspace: workspace,
				// 	path: path
				// })
				// if (alreadyExists) {
				// }
				// await RawAppService.updateRawApp({
				// 	workspace: workspace,
				// 	path: path,
				// 	requestBody: {
				// 		path: path
				// 	}
				// })
			} else if (kind == 'folder') {
				const folder = await FolderService.getFolder({
					workspace: workspace,
					name: path
				})
				return {
					name: folder.name
				}
			} else {
				throw new Error(`Unknown kind ${kind}`)
			}
		} catch {
			return {}
		}
	}

	async function showDiff(kind: Kind, path: string) {
		diffDrawer.openDrawer()
		let values = await Promise.all([
			getValue(kind, path, workspaceToDeployTo!),
			getValue(kind, path, $workspaceStore!)
		])
		diffDrawer.setDiff({
			mode: 'simple',
			original: values[0],
			current: values[1],
			title: 'Staging/prod <> Dev'
		})
	}
</script>

<div class="mt-6" />

{#if !$enterpriseLicense}
	<Alert type="error" title="Enterprise license required"
		>Deploy to staging/prod from the web UI is only available with an enterprise license</Alert
	>
{:else if notSet == true}
	<Alert type="error" title="Staging/Prod deploy not set up"
		>As an admin, go to "Workspace {'->'} Dev/Staging/Prod"</Alert
	>
{:else}
	<Alert type="info" title="Shareable page"
		>Share this <a href="/deploy/{kind}/{initialPath}">link</a> to have another properly permissioned
		user do the deployment</Alert
	>

	<h3 class="mb-2 mt-8"
		>Destination Workspace&nbsp; <Tooltip
			>Workspace to deploy to is set in the workspace settings</Tooltip
		></h3
	>
	<input class="max-w-xs" type="text" disabled value={workspaceToDeployTo} />

	{#if seeTarget == undefined}
		<div class="mt-6" />
		<Loader2 class="animate-spin" />
	{:else if seeTarget == true}
		<h3 class="mb-6 mt-16">All related deployable items</h3>

		<DiffDrawer bind:this={diffDrawer} />
		<div class="grid grid-cols-9 justify-center max-w-3xl gap-2">
			{#each dependencies ?? [] as { kind, path, include }}
				{@const statusPath = computeStatusPath(kind, path)}
				<div class="col-span-1 truncate text-secondary text-sm pt-0.5">{kind}</div><div
					class="col-span-5 truncate font-semibold">{path}</div
				><div class="col-span-1"><Toggle size="xs" bind:checked={include} /></div><div
					class="col-span-1"
				>
					{#if allAlreadyExists[statusPath] == false}
						{#if include}
							<Badge
								>New <Tooltip
									>This {kind} doesn't exist yet on the target and will be created by the deployment</Tooltip
								></Badge
							>
						{:else}
							<Badge color="red">
								Missing
								<Tooltip
									>{#if kind == 'resource_type'}
										Resource types are not re-deployed by default. We strongly recommend to add
										shared resource types in 'admin' workspace, which will have them be shared to
										every workspace.
									{:else}
										This {kind} doesn't exist and is not included in the deployment. Variables and Resources
										are considered to be workspace specific and are never included by default.
									{/if}</Tooltip
								>
							</Badge>
						{/if}
					{:else if allAlreadyExists[statusPath] == true}
						<button
							class="text-blue-600 font-normal mt-1"
							on:click={() => {
								showDiff(kind, path)
							}}>diff</button
						>
					{/if}</div
				>
				<div class="col-span-1 pr-1">
					{#if deploymentStatus[statusPath]}
						{#if deploymentStatus[statusPath].status == 'loading'}
							<Loader2 class="animate-spin" />
						{:else if deploymentStatus[statusPath].status == 'deployed'}
							<Badge color="green">Deployed</Badge>
						{:else if deploymentStatus[statusPath].status == 'failed'}
							<div class="inline-flex gap-1">
								<Badge color="red">Failed</Badge>
								<Tooltip>{deploymentStatus[statusPath].error}</Tooltip></div
							>
						{/if}
					{:else}
						<Button color="light" size="xs" on:click={() => deploy(kind, path)}>Deploy</Button>
					{/if}
				</div>
			{/each}
		</div>

		{#if !hideButton}
			<div class="mt-16 flex flex-row-reverse max-w-3xl"
				><Button on:click={deployAll}>Deploy All Toggled</Button></div
			>
		{/if}
	{:else}
		<div class="my-2" />
		<Alert type="error" title="User not allowed to deploy to this workspace"
			>Ask a permissioned user to deploy this item using the shareable link or get the proper
			permissions on the target workspace</Alert
		>
	{/if}
{/if}
