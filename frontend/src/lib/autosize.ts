export const action = (node) => {
	/* Constants */
	const update = new Event('update')

	/* Functions */
	const init = () => {
		addStyles()
		observeElement()
		addEventListeners()
		setInitialHeight()
	}

	const dispatchUpdateEvent = () => {
		node.dispatchEvent(update)
	}

	const setInitialHeight = () => {
		let height = 0

		if (node.value) {
			height = node.scrollHeight
		} else {
			if (node.placeholder) {
				node.value = node.placeholder
				height = node.scrollHeight
				node.value = ''
			} else {
				node.value = '|'
				node.style.height = '0px'
				height = node.scrollHeight
				node.value = ''
			}
		}

		node.style.height = height + 'px'
	}

	const setHeight = () => {
		node.style.height = '0px'
		node.style.height = Math.max(node.scrollHeight, 40) + 5 + 'px'
	}

	const addStyles = () => {
		node.style.boxSizing = 'border-box'
	}

	const observeElement = () => {
		let elementPrototype = Object.getPrototypeOf(node)
		let descriptor = Object.getOwnPropertyDescriptor(elementPrototype, 'value')
		Object.defineProperty(node, 'value', {
			get: function () {
				return descriptor?.get?.apply(this, arguments as any)
			},
			set: function () {
				descriptor?.set?.apply(this, arguments as any)
				dispatchUpdateEvent()
			}
		})
	}

	const addEventListeners = () => {
		node.addEventListener('input', (e) => {
			dispatchUpdateEvent()
		})
		node.addEventListener('update', setHeight)
	}

	const removeEventListeners = () => {
		node.removeEventListener('input', dispatchUpdateEvent)
		node.removeEventListener('update', setHeight)
	}

	if (node.tagName.toLowerCase() !== 'textarea') {
		throw new Error('svelte-textarea-auto-height can only be used on textarea elements.')
	} else {
		init()

		return {
			destroy() {
				removeEventListeners()
			}
		}
	}
}

export default action
