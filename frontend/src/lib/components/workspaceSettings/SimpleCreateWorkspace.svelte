<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import CreateWorkspaceInner from './CreateWorkspaceInner.svelte'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { usersWorkspaceStore } from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { sendUserToast } from '$lib/toast'
	import {
		toWorkspaceId,
		validateWorkspaceId,
		WORKSPACE_NAME_MAX_LENGTH
	} from '$lib/utils/workspaceId'
	import {
		defaultWorkspaceName,
		loadUsernamePolicy,
		WORKSPACE_HANDOVER_MS
	} from '$lib/workspaceCreation'

	interface Props {
		/** Where to go once the workspace exists. It is already the active one by then. */
		onCreated: (workspaceId: string) => void
	}

	let { onCreated }: Props = $props()

	let name = $state('')
	let creating = $state(false)

	// The full form — id, colour, username, invites — for the person who wants it. Forced on
	// when the instance does not derive usernames: one is required and a name field has
	// nowhere to ask for it.
	let advanced = $state(false)
	let automateUsername = $state(true)
	let suggestedUsername = $state<string | undefined>(undefined)

	async function load() {
		try {
			const [me, policy] = await Promise.all([UserService.globalWhoami(), loadUsernamePolicy()])
			name = defaultWorkspaceName(me.name, me.email)
			automateUsername = policy.automate
			suggestedUsername = policy.suggested
			if (!policy.automate && !policy.suggested) advanced = true
		} catch (error) {
			console.error('Could not prefill the workspace name:', error)
			name = 'My workspace'
		}
	}
	void load()

	const problem = $derived(
		!name.trim()
			? 'A name is required'
			: name.trim().length > WORKSPACE_NAME_MAX_LENGTH
				? `The name is too long (max ${WORKSPACE_NAME_MAX_LENGTH} characters).`
				: undefined
	)

	/**
	 * The id the name implies. `Bob's workspace` is Bob's, so the id is `bob` — slugifying the
	 * whole name would make `bob-s-workspace`, which is what nobody would have typed. A name
	 * that is not possessive is slugified as it stands.
	 */
	function idSeed(workspaceName: string): string {
		const owner = workspaceName.replace(/['’]s\s+workspace$/i, '').trim()
		return toWorkspaceId(owner || workspaceName) || 'workspace'
	}

	/** The id nearest that seed which nobody holds: `-2`, `-3`, … so two people named Bob both
	 *  get something readable. */
	async function freeWorkspaceId(candidate: string): Promise<string> {
		for (let n = 1; n <= 20; n++) {
			const next = n === 1 ? candidate : `${candidate}-${n}`
			if (validateWorkspaceId(next)) break
			if (!(await WorkspaceService.existsWorkspace({ requestBody: { id: next } }))) return next
		}
		return candidate
	}

	async function create() {
		if (problem || creating) return
		creating = true
		const workspaceName = name.trim()
		const started = Date.now()
		try {
			const id = await freeWorkspaceId(idSeed(workspaceName))
			await WorkspaceService.createWorkspace({
				requestBody: {
					id,
					name: workspaceName,
					username: automateUsername ? undefined : suggestedUsername
				}
			})
			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			switchWorkspace(id)
			const left = WORKSPACE_HANDOVER_MS - (Date.now() - started)
			if (left > 0) await new Promise((resolve) => setTimeout(resolve, left))
			// Left up rather than cleared: the navigation it hands over to loads the workspace
			// layout for the first time, and dropping back to the form under it would show the
			// button again for as long as that takes.
			onCreated(id)
		} catch (error) {
			console.error('Could not create the workspace:', error)
			sendUserToast('Could not create the workspace: ' + (error?.body || error?.message), true)
			creating = false
		}
	}
</script>

{#if creating}
	<div class="flex flex-col items-center gap-3 py-12 text-sm text-secondary">
		<Loader2 size={20} class="animate-spin" />
		Creating {name.trim()}…
	</div>
{:else if advanced}
	<CreateWorkspaceInner inModal onFinish={() => onCreated('')} />
{:else}
	<div class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">Workspace name</span>
		<TextInput
			bind:value={name}
			inputProps={{
				autofocus: true,
				maxlength: WORKSPACE_NAME_MAX_LENGTH,
				onkeydown: (e) => e.key === 'Enter' && create()
			}}
		/>
		{#if problem && name.trim()}
			<span class="text-2xs font-normal text-red-500">{problem}</span>
		{/if}

		<div class="mt-6 flex items-center justify-between gap-4">
			<button class="text-xs text-secondary hover:text-emphasis" onclick={() => (advanced = true)}>
				Advanced settings
			</button>
			<Button variant="accent" unifiedSize="md" disabled={!!problem} onClick={create}>
				Create workspace
			</Button>
		</div>
	</div>
{/if}
