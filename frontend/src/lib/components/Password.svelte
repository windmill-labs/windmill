<script lang="ts">
	// @ts-nocheck
	import { onMount } from 'svelte'
	export let password: string
	export let placeholder = '******'

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
</script>

<div class="relative w-full">
	<div class="absolute inset-y-0 right-0 flex items-center px-2">
		<input class="hidden js-password-toggle" id="toggle" type="checkbox" />
		<label
			class="bg-gray-300 hover:bg-gray-400 rounded px-2 py-1 text-sm text-gray-600 font-mono cursor-pointer js-password-label"
			for="toggle">show</label
		>
	</div>
	<input
		class="block w-full py-2 px-2 border rounded-md border-gray-300 shadow-sm; focus:ring focus:ring-indigo-100 focus:ring-opacity-50 text-sm js-password h-12"
		id="password"
		type="password"
		bind:value={password}
		autocomplete="off"
		{placeholder}
	/>
</div>
