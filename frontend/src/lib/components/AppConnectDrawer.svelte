<script lang="ts">
	import { run } from 'svelte/legacy'

	import { createEventDispatcher } from 'svelte'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import AppConnectInner from './AppConnectInner.svelte'
	import GoogleSigninButton from './GoogleSigninButton.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import { addResourceTitle } from './resourceTypeDisplay'

	interface Props {
		expressOAuthSetup?: boolean
		workspace?: string
		disableChatOffset?: boolean
	}

	let {
		expressOAuthSetup = false,
		workspace = undefined,
		disableChatOffset = false
	}: Props = $props()

	/** Set by `open(rt, fillPath)`, not by the parent: which resource this run fills is a
	 *  property of the click, and a prop would go stale between two different rows. */
	let fillPath: string | undefined = $state(undefined)

	let drawer: Drawer | undefined = $state()
	let resourceType = $state('')
	let step = $state(1)
	let disabled = $state(false)
	let isGoogleSignin = $state(false)
	let manual = $state(true)

	let appConnectInner: AppConnectInner | undefined = $state(undefined)

	let rtToLoad: string | undefined = $state('')
	/** `fill` connects into a resource that already exists, instead of creating one. */
	export async function open(rt?: string, fill?: string) {
		fillPath = fill
		handedOff = false
		rtToLoad = rt
		drawer?.openDrawer?.()
	}

	/**
	 * Hand off to the inner component exactly once per opening. The reactive statement below
	 * re-runs both when `rtToLoad` changes and when `appConnectInner` binds — and it binds
	 * afresh on every opening, since the drawer destroys its content on close. A second
	 * `open()` runs `next()` a second time, which walks a drawer opened on a resource type
	 * straight past the Connect button and into `window.open`; a popup opened from a reactive
	 * effect rather than from the click is blocked, leaving "Finish connection in popup
	 * window" with no popup behind it.
	 *
	 * A flag rather than the last resource type: `open()` with no argument leaves `rtToLoad`
	 * undefined, which compares equal to the initial state and would skip the hand-off
	 * entirely — the resources page opens it that way.
	 */
	let handedOff = false
	function onRtToLoadChange(rtToLoad: string | undefined) {
		if (handedOff) return
		handedOff = true
		appConnectInner?.open(rtToLoad)
	}

	const dispatch = createEventDispatcher()

	run(() => {
		appConnectInner && onRtToLoadChange(rtToLoad)
	})
</script>

<Drawer
	bind:this={drawer}
	on:close={() => {
		step = 1
		handedOff = false
		dispatch('close')
	}}
	size="700px"
	{disableChatOffset}
>
	<DrawerContent
		title={addResourceTitle(step > 1 ? resourceType : undefined)}
		id="add-resource-drawer"
		on:close={drawer?.closeDrawer}
		tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
		documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
	>
		{#snippet titleExtra()}
			{#if step > 1 && resourceType}
				<IconedResourceType name={resourceType} silent width="20px" height="20px" />
			{/if}
		{/snippet}
		<AppConnectInner
			bind:this={appConnectInner}
			bind:step
			bind:resourceType
			bind:isGoogleSignin
			bind:disabled
			bind:manual
			on:close={drawer?.closeDrawer}
			on:refresh
			express={expressOAuthSetup}
			{workspace}
			{fillPath}
		/>
		{#snippet actions()}
			<div class="flex gap-1">
				<!-- Only when the user came through the type picker: opening the drawer for one
				     resource type skips step 1, so Back would land on a list they never chose from. -->
				{#if step > 1 && !rtToLoad}
					<Button variant="default" unifiedSize="md" onClick={() => appConnectInner?.back()}>
						Back
					</Button>
				{/if}
				{#if isGoogleSignin}
					<GoogleSigninButton {disabled} onClick={() => appConnectInner?.next()} />
				{:else}
					<Button
						variant="accent"
						unifiedSize="md"
						{disabled}
						onClick={() => appConnectInner?.next()}
					>
						{#if step == 2 && !manual}
							Connect
						{:else if step == 1}
							Next
						{:else}
							Save
						{/if}
					</Button>
				{/if}
			</div>
		{/snippet}
	</DrawerContent>
</Drawer>
