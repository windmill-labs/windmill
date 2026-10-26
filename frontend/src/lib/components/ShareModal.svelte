<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'

	import { Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Toggle from './Toggle.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import ItemAccessList from './itemSettings/ItemAccessList.svelte'
	import ShareDialog, { type ShareKind } from './itemSettings/ShareDialog.svelte'

	interface Props {
		/** Where this is mounted, for the Share dialog's telemetry. */
		source?: 'list' | 'detail' | 'home'
	}

	let { source = 'list' }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

	const dispatch = createEventDispatcher()

	type Kind =
		| 'script'
		| 'group_'
		| 'resource'
		| 'schedule'
		| 'variable'
		| 'flow'
		| 'app'
		| 'raw_app'
		| 'http_trigger'
		| 'websocket_trigger'
		| 'kafka_trigger'
		| 'nats_trigger'
		| 'mqtt_trigger'
		| 'sqs_trigger'
		| 'postgres_trigger'
		| 'gcp_trigger'
		| 'azure_trigger'
		| 'amqp_trigger'
		| 'email_trigger'
		| 'volume'
	// $state so the access list tracks whichever object the drawer was last opened on. The cast
	// keeps TS from narrowing it to the initial value, which openDrawer always replaces.
	let kind: Kind = $state('script' as Kind)
	let path: string = $state('')
	let isOwnerOverride: boolean | undefined = $state()

	let drawer: Drawer | undefined = $state()
	let shareDialog: ShareDialog | undefined = $state()

	let linkedVarPaths: string[] = $state([])
	let alsoApplyToLinked: boolean = $state(true)

	function collectVarRefs(value: unknown): string[] {
		const paths: string[] = []
		function walk(v: unknown) {
			if (typeof v === 'string' && v.startsWith('$var:')) {
				paths.push(v.substring('$var:'.length))
			} else if (Array.isArray(v)) {
				v.forEach(walk)
			} else if (v && typeof v === 'object') {
				Object.values(v).forEach(walk)
			}
		}
		walk(value)
		return [...new Set(paths)]
	}

	async function loadLinkedVarPaths() {
		if (kind !== 'resource') {
			linkedVarPaths = []
			return
		}
		const currentPath = path
		try {
			const resource = await ResourceService.getResource({
				workspace: $operatingWorkspace!,
				path: currentPath
			})
			if (path !== currentPath) return
			linkedVarPaths = collectVarRefs(resource.value)
		} catch {
			if (path !== currentPath) return
			linkedVarPaths = []
		}
	}

	const SHARE_DIALOG_KINDS: Kind[] = ['flow', 'script', 'app', 'raw_app']

	export async function openDrawer(newPath: string, kind_l: Kind, isOwnerOverride_l?: boolean) {
		if (SHARE_DIALOG_KINDS.includes(kind_l)) {
			shareDialog?.openDialog(kind_l as ShareKind, newPath, { source })
			return
		}
		path = newPath
		kind = kind_l
		isOwnerOverride = isOwnerOverride_l
		alsoApplyToLinked = true
		loadLinkedVarPaths()
		drawer?.openDrawer()
	}
</script>

<ShareDialog bind:this={shareDialog} onChange={() => dispatch('change', { path, kind })} />

<Drawer bind:this={drawer}>
	<DrawerContent title="Share {path}" on:close={drawer?.closeDrawer}>
		<!-- Full height, so a long access list scrolls inside the drawer under its own header. -->
		<div class="flex flex-col gap-4 h-full">
			{#if linkedVarPaths.length > 0}
				<div class="flex flex-col gap-1.5 p-3 border rounded bg-surface-secondary text-xs">
					<Toggle
						size="xs"
						bind:checked={alsoApplyToLinked}
						options={{ right: 'Also apply to linked variables' }}
					/>
					<ul class="text-2xs text-secondary list-disc ml-4">
						{#each linkedVarPaths as varPath (varPath)}
							<li>{varPath}</li>
						{/each}
					</ul>
				</div>
			{/if}
			{#key `${kind}:${path}`}
				<ItemAccessList
					{kind}
					{path}
					alsoApplyTo={alsoApplyToLinked ? linkedVarPaths : []}
					{isOwnerOverride}
					onChange={() => dispatch('change', { path, kind })}
				/>
			{/key}
		</div>
	</DrawerContent>
</Drawer>
