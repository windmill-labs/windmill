<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import {
		parseRepoUrl,
		withToken,
		type RepoConnection,
		type TokenProvider
	} from './repoConnection'

	type Props = {
		provider: TokenProvider
		connection: RepoConnection | undefined
	}

	let { provider, connection = $bindable() }: Props = $props()

	let repoUrl = $state('')
	let token = $state('')

	const parsed = $derived(parseRepoUrl(repoUrl))
	const urlError = $derived(
		repoUrl.trim() && !parsed ? 'Enter the https URL of the repository' : ''
	)

	$effect(() => {
		connection =
			parsed && token.trim()
				? { url: parsed.toString(), tokenUrl: withToken(parsed, token, provider) }
				: undefined
	})
</script>

<div class="flex flex-col gap-4">
	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">Repository URL</span>
		<TextInput
			bind:value={repoUrl}
			error={urlError}
			inputProps={{
				placeholder:
					provider === 'github'
						? 'https://github.com/acme/windmill-workspace'
						: 'https://gitlab.com/acme/windmill-workspace'
			}}
		/>
		{#if urlError}
			<span class="text-2xs text-red-500">{urlError}</span>
		{/if}
	</label>
	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">
			{provider === 'github' ? 'Personal access token' : 'Access token'}
		</span>
		<span class="text-xs text-secondary">
			{#if provider === 'github'}
				A fine-grained token with read and write access to the repository's <b>Contents</b>.
			{:else}
				A project access token with the <code>write_repository</code> scope and the
				<code>Developer</code> role.
			{/if}
		</span>
		<TextInput bind:value={token} inputProps={{ type: 'password', autocomplete: 'off' }} />
		<span class="text-2xs text-hint">
			Stored in a secret variable next to the resource. Renew it there before it expires.
		</span>
	</label>
</div>
