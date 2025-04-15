<script lang="ts">
	import { Alert, Badge } from '$lib/components/common'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	import { HttpTriggerService } from '$lib/gen'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { isObject } from '$lib/utils'
	import { getHttpRoute } from './utils'
	import RouteBodyTransformerOption from './RouteBodyTransformerOption.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Toggle from '$lib/components/Toggle.svelte'

	export let initialTriggerPath: string | undefined = undefined
	export let dirtyRoutePath: boolean = false
	export let route_path: string | undefined
	export let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' | undefined
	export let can_write: boolean = false
	export let static_asset_config: { s3: string; storage?: string; filename?: string } | undefined =
		undefined
	export let showCapture = false
	export let headless: boolean = false
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let workspaced_route: boolean = false
	export let isValid = false
	export let runnableArgs: any = {}
	export let raw_string: boolean = false
	export let wrap_body: boolean = false
	export let capture_mode: boolean
	let validateTimeout: NodeJS.Timeout | undefined = undefined

	let routeError: string = ''
	async function validateRoute(
		routePath: string | undefined,
		method: typeof http_method,
		workspaced_route: boolean
	): Promise<void> {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!routePath || !method || !/^:?[-\w]+(\/:?[-\w]+)*$/.test(routePath)) {
				routeError = 'Endpoint not valid'
			} else if (await routeExists(routePath, method, workspaced_route)) {
				routeError = 'Endpoint already taken'
			} else {
				routeError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	async function routeExists(
		route_path: string,
		method: Exclude<typeof http_method, undefined>,
		workspaced_route: boolean
	) {
		return await HttpTriggerService.existsRoute({
			workspace: $workspaceStore!,
			requestBody: {
				route_path,
				http_method: method,
				trigger_path: initialTriggerPath,
				workspaced_route: workspaced_route
			}
		})
	}

	$: validateRoute(route_path, http_method, workspaced_route)

	$: isValid = routeError === ''

	$: fullRoute = getHttpRoute('r', route_path, workspaced_route, $workspaceStore ?? '')

	$: !http_method && (http_method = 'post')
	$: route_path === undefined && (route_path = '')
</script>

<div>
	{#if showCapture && captureInfo}
		{@const captureURL = `${location.origin}${base}/api/w/${$workspaceStore}/capture_u/http/${
			captureInfo.isFlow ? 'flow' : 'script'
		}/${captureInfo.path.replaceAll('/', '.')}/${route_path}`}
		{@const cleanedRunnableArgs =
			isObject(runnableArgs) && 'wm_trigger' in runnableArgs
				? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
				: runnableArgs}
		<CaptureSection
			captureType="http"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		>
			<Label label="URL">
				<ClipboardPanel content={captureURL} disabled={!captureInfo.active} />
			</Label>

			<Label label="Example cUrl">
				<CopyableCodeBlock
					disabled={!captureInfo.active}
					code={`curl \\
-X ${(http_method ?? 'post').toUpperCase()} ${captureURL} \\
-H 'Content-Type: application/json' \\
-d '${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)}'`}
					language={bash}
				/>
			</Label>
		</CaptureSection>
	{/if}
	<Section label="HTTP" {headless}>
		{#if !($userStore?.is_admin || $userStore?.is_super_admin)}
			<Alert type="info" title="Admin only" collapsible>
				Route endpoints can only be edited by workspace admins
			</Alert>
			<div class="my-2"></div>
		{/if}
		<div class="flex flex-col w-full gap-4">
			<label class="block grow w-full">
				<div class="flex flex-col gap-1">
					<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
						<div>
							Path
							<Required required={true} />
						</div>
						<div class="text-2xs text-tertiary"> ':myparam' for path params </div>
					</div>
					<!-- svelte-ignore a11y-autofocus -->
					<input
						type="text"
						autocomplete="off"
						bind:value={route_path}
						disabled={!($userStore?.is_admin || $userStore?.is_super_admin) || !can_write}
						class={routeError === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
						on:input={() => {
							dirtyRoutePath = true
						}}
					/>
				</div>
			</label>

			<ToggleButtonGroup
				class="w-auto"
				bind:selected={http_method}
				disabled={!($userStore?.is_admin || $userStore?.is_super_admin) ||
					!can_write ||
					!!static_asset_config}
				let:item
			>
				<ToggleButton label="GET" value="get" {item} />
				<ToggleButton label="POST" value="post" {item} />
				<ToggleButton label="PUT" value="put" {item} />
				<ToggleButton label="PATCH" value="patch" {item} />
				<ToggleButton label="DELETE" value="delete" {item} />
			</ToggleButtonGroup>
			<div class="flex flex-col w-full mt-2">
				<div class="flex justify-start w-full">
					<Badge
						color="gray"
						class="center-center !bg-surface-secondary !text-tertiary !w-[90px] !h-[24px] rounded-r-none border"
					>
						Full endpoint
					</Badge>
					<input
						type="text"
						readonly
						value={fullRoute}
						size={fullRoute.length || 50}
						class="font-mono !text-xs max-w-[calc(100%-70px)] !w-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
						on:focus={({ currentTarget }) => {
							currentTarget.select()
						}}
					/>
				</div>
				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
					>{dirtyRoutePath ? routeError : ''}</div
				>
				{#if !capture_mode && !isCloudHosted()}
					<div class="mt-1">
						<Toggle
							size="sm"
							checked={workspaced_route}
							on:change={() => {
								workspaced_route = !workspaced_route
								dirtyRoutePath = true
							}}
							options={{
								right: 'Prefix with workspace',
								rightTooltip:
									'Prefixes the route with the workspace ID (e.g., {base_url}/api/r/{workspace_id}/{route}). Note: deploying the HTTP trigger to another workspace updates the route workspace prefix accordingly.',
								rightDocumentationLink:
									'https://www.windmill.dev/docs/core_concepts/http_routing#workspace-prefix'
							}}
						/>
					</div>
				{/if}
			</div>
			{#if capture_mode}
				<RouteBodyTransformerOption bind:raw_string bind:wrap_body />
			{/if}
		</div>
	</Section>
</div>
