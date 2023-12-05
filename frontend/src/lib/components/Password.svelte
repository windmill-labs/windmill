<script lang="ts">
	// @ts-nocheck
	import { onMount } from 'svelte'
	export let password: string
	export let placeholder = '******'
	export let disabled = false
	export let required = false

	onMount(() => {
		const passwordToggle = document.querySelector('.js-password-toggle')
		if (passwordToggle) {
			passwordToggle.addEventListener('change', function () {
				const password = document.querySelector('.js-password'),
					passwordLabel = document.querySelector('.js-password-label')

				if (password.type === 'password') {
					password.type = 'text'
					passwordLabel.innerHTML = 'hide'
				} else {
					password.type = 'password'
					passwordLabel.innerHTML = 'show'
				}

				password.focus()
			})
		} else {
			throw Error('Password component is undefined')
		}
	})

	$: red = required && (password == '' || password == undefined)
</script>

<div class="relative w-full">
	<div class="absolute inset-y-0 right-0 flex items-center px-2">
		<input class="!hidden js-password-toggle" id="toggle" type="checkbox" />
		<label
			class="bg-surface-secondary hover:bg-gray-400 rounded px-2 py-1 text-sm text-tertiary font-mono cursor-pointer js-password-label"
			for="toggle">show</label
		>
	</div>
	<input
		class="block w-full px-2 py-1 {red ? '!border-red-500' : ''} text-sm js-password h-9"
		id="password"
		type="password"
		bind:value={password}
		autocomplete="off"
		{placeholder}
		{disabled}
	/>
</div>
{#if red}
	<div class="text-red-600 text-2xs grow">This field is required</div>
{/if}
