<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import TokensTable from '$lib/components/settings/TokensTable.svelte'

	interface Props {
		open?: boolean
		/** Given, the drawer mints a token restricted to these scopes, for the caller to use. */
		scopes?: string[]
		newTokenLabel?: string
		newTokenWorkspace?: string
		newToken?: string
		onTokenCreated?: (token: string) => void
	}

	let {
		open = $bindable(),
		scopes = undefined,
		newTokenLabel = undefined,
		newTokenWorkspace = undefined,
		newToken = $bindable(undefined),
		onTokenCreated
	}: Props = $props()

	export function openDrawer() {
		open = true
	}

	export function closeDrawer() {
		open = false
	}
</script>

<Drawer bind:open size="900px">
	<DrawerContent title="Tokens" on:close={closeDrawer}>
		<div class="pb-8">
			<TokensTable
				defaultNewTokenLabel={newTokenLabel}
				defaultNewTokenWorkspace={newTokenWorkspace}
				{scopes}
				onTokenCreated={(token) => {
					newToken = token
					onTokenCreated?.(token)
				}}
			/>
		</div>
	</DrawerContent>
</Drawer>
