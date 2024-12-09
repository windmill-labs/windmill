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
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { base } from '$lib/base'

	export let args: Record<string, any> = { route_path: '', http_method: 'get' }
	export let routeError: string = ''
	export let dirtyRoutePath: boolean = false
	export let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' = 'post'
	export let can_write: boolean = false
	export let static_asset_config: { s3: string; storage?: string; filename?: string } | undefined =
		undefined
	export let showCapture = false
	export let initialRoutePath: string = ''
	export let isFlow = true
	export let path: string = ''
	export let headless: boolean = false

	let validateTimeout: NodeJS.Timeout | undefined = undefined

	async function validateRoute(path: string, method: typeof http_method): Promise<void> {
		routeError = ''
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!/^[\w-:]+(\/[\w-:]+)*$/.test(path)) {
				routeError = 'Endpoint not valid'
			} else if (initialRoutePath !== path && (await routeExists(path, method))) {
				routeError = 'Endpoint already taken'
			} else {
				routeError = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	async function routeExists(route_path: string, method: typeof http_method) {
		return await HttpTriggerService.existsRoute({
			workspace: $workspaceStore!,
			requestBody: {
				route_path,
				http_method
			}
		})
	}

	function getHttpRoute(route_path: string | undefined, capture: boolean) {
		if (capture) {
			return `${$page.url.origin}/api/w/${$workspaceStore}/capture_u/http/${
				isFlow ? 'flow' : 'script'
			}/${path.replaceAll('/', '.')}/${route_path ?? ''}`
		} else {
			return `${$page.url.origin}${base}/api/r/${
				isCloudHosted() ? $workspaceStore + '/' : ''
			}${route_path}`
		}
	}

	$: validateRoute(route_path, http_method)

	$: fullRoute = getHttpRoute(route_path, showCapture)

	$: showCapture && (http_method = 'post')

	export let route_path = ''

	function updateArgs(route_path: string, http_method: string) {
		args && ((args.route_path = route_path), (args.http_method = http_method))
	}

	$: updateArgs(route_path, http_method)
</script>

<Section label="HTTP" {headless}>
	{#if !($userStore?.is_admin || $userStore?.is_super_admin)}
		<Alert type="info" title="Admin only" collapsible>
			Route endpoints can only be edited by workspace admins
		</Alert>
		<div class="my-2" />
	{/if}
	<div class="flex flex-col w-full gap-4">
		<label class="block grow w-full">
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
		</label>

		<ToggleButtonGroup
			class="w-auto"
			bind:selected={http_method}
			disabled={!($userStore?.is_admin || $userStore?.is_super_admin) ||
				!can_write ||
				!!static_asset_config ||
				showCapture}
		>
			<ToggleButton label="GET" value="get" />
			<ToggleButton label="POST" value="post" />
			<ToggleButton label="PUT" value="put" />
			<ToggleButton label="PATCH" value="patch" />
			<ToggleButton label="DELETE" value="delete" />
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
		</div>
		{#if showCapture}
			<Label label="cUrl">
				<CopyableCodeBlock
					code={`curl \\
-X POST ${fullRoute} \\
-H 'Content-Type: application/json' \\
-d '{"foo": 42}'`}
					language={bash}
				/>
			</Label>
		{/if}
	</div>
</Section>
