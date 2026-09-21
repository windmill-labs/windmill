<script lang="ts">
	import { GitSyncService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Popover from '../meltComponents/Popover.svelte'
	import Button from '../common/button/Button.svelte'
	import { Alert } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import { KeyRound, Loader2 } from 'lucide-svelte'

	interface Props {
		workspace: string
		/** The repository as currently saved, which is the key the token is stored
		 * under. A URL edited but not yet saved would file the token against a
		 * repository the resource does not point at, so the caller disables this
		 * until it is saved. */
		repoUrl: string
		disabled?: boolean
		onReplaced?: () => void
	}

	let { workspace, repoUrl, disabled = false, onReplaced }: Props = $props()

	let token = $state('')
	let saving = $state(false)
	let error: string | undefined = $state(undefined)

	/** The instance and project a repository URL names, for checking the pasted
	 * token against the repository it is meant for. */
	function repoParts(url: string): { base: string; project: string } | undefined {
		try {
			const u = new URL(url)
			const project = u.pathname.replace(/^\/+/, '').replace(/\.git$/, '')
			return project ? { base: `${u.protocol}//${u.host}`, project } : undefined
		} catch {
			return undefined
		}
	}

	async function replace(close: (_: any) => void) {
		if (!token || saving) return
		saving = true
		error = undefined
		// Pinned before the first await, all three: the field stays editable and the
		// props follow the drawer's selected workspace and deployed URL, so
		// re-reading any of them afterwards would store the token against something
		// the check never validated.
		const candidate = token
		const forRepo = repoUrl
		const inWorkspace = workspace
		try {
			// Check the token before storing it. The server binds a credential to its
			// repository but only refuses it when something tries to use it, so a
			// wrong token would otherwise be accepted here and surface as a failed
			// sync later.
			// A URL that does not parse names no project to check, and would be
			// stored as the credential's key verbatim: a `$var:` reference here
			// keys the token to a repository that does not exist.
			const parts = repoParts(forRepo)
			if (!parts) {
				error = 'The resource URL must name the repository directly to replace its token here.'
				return
			}
			const projects = await GitSyncService.listGitlabProjects({
				workspace: inWorkspace,
				// Searched by name rather than listed whole: the listing is one
				// capped page, so a token that reaches more projects than fit
				// would not show this one and a working token would be refused.
				requestBody: {
					base_url: parts.base,
					token: candidate,
					search: parts.project.split('/').pop()
				}
			})
			if (!projects.some((p) => p.path_with_namespace === parts.project)) {
				error = `That token cannot push to ${parts.project}. Check its role and that it belongs to this project.`
				return
			}
			await GitSyncService.setGitCredential({
				workspace: inWorkspace,
				requestBody: { repo_url: forRepo, token: candidate }
			})
			token = ''
			sendUserToast('Token replaced')
			onReplaced?.()
			close(null)
		} catch (err) {
			error = err?.body ?? err?.message ?? String(err)
		} finally {
			saving = false
		}
	}
</script>

<Popover contentClasses="overflow-auto" {disabled}>
	{#snippet trigger()}
		<Button
			variant="default"
			unifiedSize="xs"
			{disabled}
			startIcon={{ icon: KeyRound }}
			nonCaptureEvent
		>
			Replace token
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<div class="block text-primary p-4">
			<div class="flex flex-col gap-3 w-[420px]">
				<div class="flex flex-col gap-y-1">
					<div class="text-xs font-semibold text-emphasis">New access token</div>
					<div class="text-xs font-normal text-secondary">
						For the same repository. Windmill stores it in place of the current one and renews it
						from then on.
					</div>
					<TextInput bind:value={token} size="sm" inputProps={{ type: 'password' }} />
				</div>
				{#if error}
					<Alert type="error" title="Could not replace the token" size="xs">{error}</Alert>
				{/if}
				<div class="flex justify-end">
					<Button
						variant="accent"
						unifiedSize="sm"
						disabled={!token || saving}
						startIcon={{ icon: saving ? Loader2 : KeyRound, classes: saving ? 'animate-spin' : '' }}
						onclick={() => replace(close)}
					>
						Replace
					</Button>
				</div>
			</div>
		</div>
	{/snippet}
</Popover>
