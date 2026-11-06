<script lang="ts">
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import {
		isDefaultSuperadminActive,
		REPLACE_DEFAULT_SUPERADMIN_HREF
	} from '$lib/defaultSuperadmin'

	// Mount only for superadmins: the check is a superadmin endpoint. No dismiss: it stays
	// for as long as anyone reaching the instance can sign in as a superadmin with the
	// published credentials. Replacing the account ends the session, so the next login
	// remounts the shell and re-checks.
	let active = $state(false)
	isDefaultSuperadminActive().then((v) => (active = v))

	const palette = alertClasses.error
	const Icon = alertIcons.error
</script>

{#if active}
	<div
		class="shrink-0 px-4 py-1.5 flex items-center justify-center gap-x-3 gap-y-1 flex-wrap {palette.bgClass}"
		role="alert"
	>
		<Icon size={16} class="shrink-0 {palette.iconClass}" />
		<span class="text-xs font-medium {palette.titleClass}">
			The default superadmin account admin@windmill.dev / changeme is still active. Anyone who can
			reach this instance can log in with it.
		</span>
		<Button variant="default" unifiedSize="xs" href={REPLACE_DEFAULT_SUPERADMIN_HREF}>
			Replace account
		</Button>
	</div>
{/if}
