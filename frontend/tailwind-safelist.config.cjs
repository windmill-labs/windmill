/** @type {import('tailwindcss').Config} */
const config = {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	safelist: [
		'hljs',
		'splitpanes__pane',
		'splitpanes__splitter',
		'wm-tab',
		'autocomplete-list',
		'autocomplete-list-item',
		'autocomplete-list-item-create',
		'selected',
		'wm-tab-selected',
		...(process.env.NODE_ENV === 'production'
			? [
					{ pattern: /^m(\w?)-.*$/ },
					{ pattern: /^p(\w?)-.*$/ },
					{ pattern: /^rounded-.*$/ },
					{ pattern: /^shadow-.*$/, variants: ['hover'] },
					{ pattern: /^text-[^/]*$/, variants: ['hover', 'active', 'focus'] },
					{ pattern: /^bg-[^/]*$/, variants: ['hover', 'active', 'focus'] },
					{ pattern: /^border-[^/]*$/, variants: ['hover', 'active', 'focus'] },
					{ pattern: /^ring-[^/]*$/, variants: ['hover', 'active', 'focus'] }
			  ]
			: [])
	]
}

module.exports = config
