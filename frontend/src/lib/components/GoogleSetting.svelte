<script lang="ts">
	import CollapseLink from './CollapseLink.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'

	interface Props {
		value: any
		baseUrl?: string
	}

	let { value = $bindable(), baseUrl }: Props = $props()

	// Google OAuth endpoints are fixed
	const GOOGLE_AUTH_URL = 'https://accounts.google.com/o/oauth2/v2/auth'
	const GOOGLE_TOKEN_URL = 'https://oauth2.googleapis.com/token'
	const GOOGLE_USERINFO_URL = 'https://www.googleapis.com/oauth2/v3/userinfo'

	// Scopes needed for Drive and Calendar triggers
	const GOOGLE_SCOPES = [
		'https://www.googleapis.com/auth/drive.readonly',
		'https://www.googleapis.com/auth/calendar.readonly',
		'https://www.googleapis.com/auth/calendar.events'
	]

	let enabled = $derived(value != undefined)

	function initializeValue() {
		value = {
			id: '',
			secret: '',
			display_name: 'Google',
			login_config: {
				auth_url: GOOGLE_AUTH_URL,
				token_url: GOOGLE_TOKEN_URL,
				userinfo_url: GOOGLE_USERINFO_URL,
				scopes: GOOGLE_SCOPES
			},
			connect_config: {
				auth_url: GOOGLE_AUTH_URL,
				token_url: GOOGLE_TOKEN_URL,
				scopes: GOOGLE_SCOPES
			}
		}
	}
</script>

<div class="flex flex-col gap-1">
	<!-- svelte-ignore a11y_label_has_associated_control -->
	<label class="text-xs font-semibold text-emphasis flex gap-4 items-center">
		<div class="w-[120px]"><IconedResourceType name="google" after={true} /></div>
		<Toggle
			checked={enabled}
			on:change={(e) => {
				if (e.detail) {
					initializeValue()
				} else {
					value = undefined
				}
			}}
		/>
	</label>
	{#if enabled}
		<div class="p-4 rounded-md border flex flex-col gap-6">
			<div class="text-xs text-tertiary">
				Configure Google OAuth to enable Google Drive and Calendar triggers. Both services share the same OAuth credentials.
			</div>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Custom Name</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'Google' }}
					bind:value={value['display_name']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client ID</span>
				<span class="text-secondary font-normal text-xs">
					OAuth 2.0 Client ID from Google Cloud Console
				</span>
				<TextInput
					inputProps={{ type: 'text', placeholder: 'xxxx.apps.googleusercontent.com' }}
					bind:value={value['id']}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Client Secret</span>
				<span class="text-secondary font-normal text-xs">
					OAuth 2.0 Client Secret from Google Cloud Console
				</span>
				<TextInput
					inputProps={{ type: 'password', placeholder: 'Client Secret' }}
					bind:value={value['secret']}
				/>
			</label>
			<div class="flex flex-col gap-1">
				<span class="text-emphasis font-semibold text-xs">Required Scopes</span>
				<div class="text-xs text-tertiary font-mono bg-surface-secondary p-2 rounded">
					{#each GOOGLE_SCOPES as scope}
						<div>{scope}</div>
					{/each}
				</div>
			</div>
			<CollapseLink text="Setup Instructions">
				<div class="text-xs text-primary border rounded-md p-4 flex flex-col gap-2">
					<p><strong>1.</strong> Go to the <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener" class="text-blue-500 underline">Google Cloud Console - Credentials</a></p>
					<p><strong>2.</strong> Create a new project or select an existing one</p>
					<p><strong>3.</strong> Click "Create Credentials" and select "OAuth 2.0 Client IDs"</p>
					<p><strong>4.</strong> Configure the OAuth consent screen if prompted</p>
					<p><strong>5.</strong> Set application type to "Web application"</p>
					<p><strong>6.</strong> Add the following redirect URI:</p>
					<code class="block bg-surface-secondary p-2 rounded mt-1">{baseUrl || 'BASE_URL'}/user/login_callback/google</code>
					<p><strong>7.</strong> Copy the Client ID and Client Secret to the fields above</p>
					<p><strong>8.</strong> Enable the following APIs in your Google Cloud project:</p>
					<ul class="list-disc ml-4">
						<li>Google Drive API</li>
						<li>Google Calendar API</li>
					</ul>
				</div>
			</CollapseLink>
		</div>
	{/if}
</div>
