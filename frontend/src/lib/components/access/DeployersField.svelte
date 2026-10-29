<script lang="ts">
	import { Button } from '$lib/components/common'
	import { GranularAclService, type PrincipalAccess } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import DeployersPicker from './DeployersPicker.svelte'

	let {
		principal,
		access
	}: {
		/** `u/<service account>` */
		principal: string
		access: PrincipalAccess
	} = $props()

	let saved: string[] = $state([])
	let deployers: string[] = $state([])
	let saving = $state(false)
	let loaded = $state(false)

	async function load(): Promise<void> {
		const list = await GranularAclService.getServiceAccountDeployers({
			workspace: $workspaceStore!,
			principal
		})
		saved = list.deployers
		deployers = [...list.deployers]
		loaded = true
	}
	load().catch((e) => sendUserToast(`Could not load deployers: ${e?.body ?? e}`, true))

	const dirty = $derived(
		deployers.length !== saved.length || deployers.some((d) => !saved.includes(d))
	)
	const reads = $derived(access.items.map((i) => i.path))

	async function save(): Promise<void> {
		saving = true
		try {
			await GranularAclService.setServiceAccountDeployers({
				workspace: $workspaceStore!,
				requestBody: { principal, deployers }
			})
			saved = [...deployers]
			sendUserToast(`Saved who can deploy as ${principal}`)
		} catch (e) {
			sendUserToast(`Could not save: ${e?.body ?? e}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="flex flex-col gap-2">
	<p class="text-2xs text-secondary">
		These people can deploy scripts, flows, apps, schedules and triggers that run as {principal}.
		Workspace admins and members of wm_deployers always can. Anyone else who deploys an item set to
		run as {principal} makes it run as themselves.
	</p>
	{#if loaded}
		<DeployersPicker {principal} bind:deployers {reads} />
		<div>
			<Button variant="accent" unifiedSize="sm" disabled={!dirty} loading={saving} onClick={save}>
				Save
			</Button>
		</div>
	{/if}
</div>
