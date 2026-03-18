<script lang="ts">
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	// Ensure the nested object exists
	if (!$values['github_enterprise_app']) {
		$values['github_enterprise_app'] = {}
	}

	let selfManaged = $derived(!!$values['github_enterprise_app'].self_managed)
	let fieldsDisabled = $derived(disabled || !selfManaged)
</script>

<div class="space-y-6">
	<Toggle
		size="xs"
		options={{ right: 'Self-managed GitHub App (for GHES or custom GitHub App)' }}
		checked={selfManaged}
		on:change={() => {
			$values['github_enterprise_app'] = {
				...$values['github_enterprise_app'],
				self_managed: !selfManaged
			}
		}}
	/>

	{#if !selfManaged}
		<p class="text-xs text-secondary">
			Using the managed Windmill GitHub App via stats.windmill.dev. Enable self-managed mode to
			configure your own GitHub App (required for GitHub Enterprise Server).
		</p>
	{:else}
		<details class="mt-1">
			<summary class="text-xs font-medium text-secondary cursor-pointer hover:text-primary"
				>How to create a GitHub App</summary
			>
			<div class="mt-2 p-3 bg-surface rounded text-2xs text-secondary space-y-2">
				<p>
					<strong>1.</strong> On your GitHub instance, go to
					<strong>Settings &rarr; Developer settings &rarr; GitHub Apps &rarr; New GitHub App</strong
					>.
				</p>
				<p><strong>2.</strong> Fill in the required fields:</p>
				<ul class="list-disc ml-4 space-y-1">
					<li>
						<strong>GitHub App name</strong>: e.g. <code>windmill-sync</code> (this becomes the app
						slug)
					</li>
					<li>
						<strong>Homepage URL</strong>: your Windmill instance URL
					</li>
					<li>
						<strong>Callback URL</strong>: <code>&lt;your-windmill-url&gt;/gh_success</code>
					</li>
					<li>
						<strong>Setup URL</strong> (optional):
						<code>&lt;your-windmill-url&gt;/gh_success</code> with "Redirect on update" checked
					</li>
					<li>Uncheck <strong>Active</strong> under Webhook (not needed)</li>
				</ul>
				<p><strong>3.</strong> Set repository permissions:</p>
				<ul class="list-disc ml-4 space-y-1">
					<li><strong>Contents</strong>: Read &amp; write</li>
					<li><strong>Metadata</strong>: Read-only</li>
				</ul>
				<p>
					<strong>4.</strong> Under "Where can this GitHub App be installed?", choose
					<strong>Any account</strong> (or restrict to your organization).
				</p>
				<p>
					<strong>5.</strong> Click <strong>Create GitHub App</strong>. On the next page, note the
					<strong>App ID</strong> and <strong>Client ID</strong>.
				</p>
				<p>
					<strong>6.</strong> Scroll down and click <strong>Generate a private key</strong>. Save the
					downloaded <code>.pem</code> file — paste its contents into the Private Key field below.
				</p>
				<p>
					<strong>7.</strong> The <strong>App Slug</strong> is the URL-friendly name shown in the
					app's URL (e.g. <code>github.com/apps/<strong>windmill-sync</strong></code
					>).
				</p>
				<p>
					<strong>8.</strong> The <strong>Base URL</strong> is your GitHub instance root (e.g.
					<code>https://github.com</code> or <code>https://github.mycompany.com</code>).
				</p>
			</div>
		</details>
	{/if}

	<div class="grid grid-cols-2 gap-x-2 gap-y-6">
		<div class="flex flex-col gap-1">
			<label for="ghes_base_url" class="block text-xs font-semibold text-emphasis mb-1">
				Base URL
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_base_url',
					placeholder: 'https://github.mycompany.com',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].base_url}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_app_id" class="block text-xs font-semibold text-emphasis mb-1">
				App ID
			</label>
			<TextInput
				inputProps={{
					type: 'number',
					id: 'ghes_app_id',
					placeholder: '12345',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].app_id}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_app_slug" class="block text-xs font-semibold text-emphasis mb-1">
				App Slug
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_app_slug',
					placeholder: 'my-windmill-app',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].app_slug}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="ghes_client_id" class="block text-xs font-semibold text-emphasis mb-1">
				Client ID
			</label>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'ghes_client_id',
					placeholder: 'Iv1.abc123',
					disabled: fieldsDisabled
				}}
				bind:value={$values['github_enterprise_app'].client_id}
			/>
		</div>
	</div>

	<div class="flex flex-col gap-1">
		<label for="ghes_private_key" class="block text-xs font-semibold text-emphasis mb-1">
			Private Key (PEM)
		</label>
		<textarea
			id="ghes_private_key"
			class="w-full h-32 font-mono text-xs p-2 border rounded resize-y {fieldsDisabled
				? 'bg-surface-disabled text-disabled cursor-not-allowed'
				: 'bg-surface text-primary'}"
			placeholder="-----BEGIN RSA PRIVATE KEY-----&#10;...&#10;-----END RSA PRIVATE KEY-----"
			disabled={fieldsDisabled}
			bind:value={$values['github_enterprise_app'].private_key}
		></textarea>
	</div>
</div>
