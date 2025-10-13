<script module>
	import { tick } from 'svelte'

	export function portal(el, options) {
		let { target, name } = options
		let targetEl
		async function update(newTarget) {
			target = newTarget
			if (typeof target === 'string') {
				targetEl = document.querySelector(target)
				if (targetEl === null) {
					await tick()
					targetEl = document.querySelector(target)
				}
				if (targetEl === null) {
					throw new Error(`No element found matching css selector: "${target}"`)
				}
			} else if (target instanceof HTMLElement) {
				targetEl = target
			} else {
				throw new TypeError(
					`Unknown portal target type: ${
						target === null ? 'null' : typeof target
					}. Allowed types: string (CSS selector) or HTMLElement.`
				)
			}
			if (!el.classList.contains('windmill-app')) {
				el.classList.add('windmill-app')
			}
			if (name && !el.classList.contains(name)) {
				el.classList.add(name)
			}
			targetEl.appendChild(el)
			el.hidden = false
		}

		function destroy() {
			if (el.parentNode) {
				el.parentNode.removeChild(el)
			}
		}

		update(target)
		return {
			update,
			destroy
		}
	}
</script>

<script>
	/**
	 * @typedef {Object} Props
	 * @property { HTMLElement|string} [target] - DOM Element or CSS Selector
	 * @property {any} [name]
	 * @property {string|undefined} [class]
	 * @property {import('svelte').Snippet} [children]
	 */

	/** @type {Props} */
	let { target = 'body', name = undefined, class: clazz = undefined, children } = $props()
</script>

<div use:portal={{ target, name }} hidden class={clazz}>
	{@render children?.()}
</div>
