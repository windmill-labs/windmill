const plugin = require('tailwindcss/plugin')
const {
	tailwindClasses
} = require('./src/lib/components/apps/editor/componentsPanel/tailwindUtils')
const { zIndexes } = require('./src/lib/zIndexes')

const figmaTokens = makeRgb(require('./src/lib/assets/tokens/tokens.json'))
const { darkModeName, lightModeName } = require('./src/lib/assets/tokens/colorTokensConfig')

const tokens = { dark: figmaTokens.tokens[darkModeName], light: figmaTokens.tokens[lightModeName] }
const primitives = figmaTokens.primitives.light

// Helper function to create color definition based on whether the value contains alpha
function createColorDefinition(colorName) {
	// Check a sample value to see if it contains alpha (using light theme as reference)
	const sampleValue = tokens.light[colorName]
	if (sampleValue && sampleValue.includes('/')) {
		// Alpha is already included, use as-is without allowing override
		return `rgb(var(--color-${colorName}))`
	} else {
		// No alpha, allow Tailwind's alpha-value placeholder
		return `rgb(var(--color-${colorName}) / <alpha-value>)`
	}
}

/** @type {import('tailwindcss').Config} */
const config = {
	//to generatd tailwind full, only include ./src/lib/components/apps/editor/componentsPanel/tailwindUtils.ts and run:
	//npx tailwindcss -i src/lib/assets/app.css -o static/tailwind_full.css
	//then copy content in that file
	content: [
		'./src/**/*.{html,js,svelte,ts}',
		'!./src/lib/components/apps/utils.ts',
		'!./src/lib/components/apps/editor/componentsPanel/tailwindUtils.ts'
	],
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
		...Object.values(zIndexes).map((zIndex) => `z-[${zIndex}]`),
		...tailwindClasses
	],
	theme: {
		colors: {
			current: 'currentcolor',
			transparent: 'transparent',
			white: '#ffffff',
			black: '#000000',

			// Surface colors (semantic)
			surface: createColorDefinition('surface-primary'),
			'surface-secondary': createColorDefinition('surface-secondary'),
			'surface-sunken': createColorDefinition('surface-sunken'),
			'surface-tertiary': createColorDefinition('surface-tertiary'),
			'surface-hover': createColorDefinition('surface-hover'),
			'surface-disabled': createColorDefinition('surface-disabled'),
			'surface-selected': createColorDefinition('surface-selected'),
			'surface-accent-primary': createColorDefinition('surface-accent-primary'),
			'surface-accent-hover': createColorDefinition('surface-accent-hover'),
			'surface-accent-clicked': createColorDefinition('surface-accent-clicked'),
			'surface-accent-secondary': createColorDefinition('surface-accent-secondary'),
			'surface-accent-secondary-hover': createColorDefinition('surface-accent-secondary-hover'),
			'surface-accent-secondary-clicked': createColorDefinition('surface-accent-secondary-clicked'),
			'surface-accent-selected': createColorDefinition('surface-accent-selected'),
			'surface-input': createColorDefinition('surface-input'),

			// Text colors (semantic)
			primary: createColorDefinition('text-primary'),
			secondary: createColorDefinition('text-secondary'),
			tertiary: createColorDefinition('text-tertiary'),
			hint: createColorDefinition('text-hint'),
			disabled: createColorDefinition('text-disabled'),
			emphasis: createColorDefinition('text-emphasis'),
			accent: createColorDefinition('text-accent'),

			// Inverse colors (computed from opposite theme)
			'primary-inverse': createColorDefinition('text-primary-inverse'),
			'secondary-inverse': createColorDefinition('text-secondary-inverse'),
			'tertiary-inverse': createColorDefinition('text-tertiary-inverse'),
			'emphasis-inverse': createColorDefinition('text-emphasis-inverse'),
			'hint-inverse': createColorDefinition('text-hint-inverse'),
			'disabled-inverse': createColorDefinition('text-disabled-inverse'),

			// Border colors (semantic)
			'border-light': createColorDefinition('border-light'),
			'border-normal': createColorDefinition('border-normal'),
			'border-accent': createColorDefinition('border-accent'),
			'border-selected': createColorDefinition('border-selected'),

			// Reserved colors (semantic)
			ai: createColorDefinition('reserved-ai'),
			'ai-inverse': createColorDefinition('reserved-ai-inverse'),

			// Surface inverse colors (computed from opposite theme)
			'surface-inverse': createColorDefinition('surface-primary-inverse'),
			'surface-secondary-inverse': createColorDefinition('surface-secondary-inverse'),
			'surface-hover-inverse': createColorDefinition('surface-hover-inverse'),
			'surface-disabled-inverse': createColorDefinition('surface-disabled-inverse'),
			'surface-selected-inverse': createColorDefinition('surface-selected-inverse'),

			// Component colors
			'component-virtual-node': createColorDefinition('component-virtual-node'),

			// Design token colors
			'light-blue': `rgb(${primitives['light-blue']})`,
			'deep-blue': {
				50: `rgb(${primitives['deep-blue-50']})`,
				100: `rgb(${primitives['deep-blue-100']})`,
				200: `rgb(${primitives['deep-blue-200']})`,
				300: `rgb(${primitives['deep-blue-300']})`,
				400: `rgb(${primitives['deep-blue-400']})`,
				500: `rgb(${primitives['deep-blue-500']})`,
				600: `rgb(${primitives['deep-blue-600']})`,
				700: `rgb(${primitives['deep-blue-700']})`,
				800: `rgb(${primitives['deep-blue-800']})`,
				900: `rgb(${primitives['deep-blue-900']})`
			},
			red: {
				50: `rgb(${primitives['red-50']})`,
				100: `rgb(${primitives['red-100']})`,
				200: `rgb(${primitives['red-200']})`,
				300: `rgb(${primitives['red-300']})`,
				400: `rgb(${primitives['red-400']})`,
				500: `rgb(${primitives['red-500']})`,
				600: `rgb(${primitives['red-600']})`,
				700: `rgb(${primitives['red-700']})`,
				800: `rgb(${primitives['red-800']})`,
				900: `rgb(${primitives['red-900']})`,
				950: `rgb(${primitives['red-950']})`
			},
			blue: {
				50: `rgb(${primitives['blue-50']})`,
				100: `rgb(${primitives['blue-100']})`,
				200: `rgb(${primitives['blue-200']})`,
				300: `rgb(${primitives['blue-300']})`,
				400: `rgb(${primitives['blue-400']})`,
				500: `rgb(${primitives['blue-500']})`,
				600: `rgb(${primitives['blue-600']})`,
				700: `rgb(${primitives['blue-700']})`,
				800: `rgb(${primitives['blue-800']})`,
				900: `rgb(${primitives['blue-900']})`,
				950: `rgb(${primitives['blue-950']})`
			},
			green: {
				50: `rgb(${primitives['green-50']})`,
				100: `rgb(${primitives['green-100']})`,
				200: `rgb(${primitives['green-200']})`,
				300: `rgb(${primitives['green-300']})`,
				400: `rgb(${primitives['green-400']})`,
				500: `rgb(${primitives['green-500']})`,
				600: `rgb(${primitives['green-600']})`,
				700: `rgb(${primitives['green-700']})`,
				800: `rgb(${primitives['green-800']})`,
				900: `rgb(${primitives['green-900']})`,
				950: `rgb(${primitives['green-950']})`
			},
			orange: {
				50: `rgb(${primitives['orange-50']})`,
				100: `rgb(${primitives['orange-100']})`,
				200: `rgb(${primitives['orange-200']})`,
				300: `rgb(${primitives['orange-300']})`,
				400: `rgb(${primitives['orange-400']})`,
				500: `rgb(${primitives['orange-500']})`,
				600: `rgb(${primitives['orange-600']})`,
				700: `rgb(${primitives['orange-700']})`,
				800: `rgb(${primitives['orange-800']})`,
				900: `rgb(${primitives['orange-900']})`,
				950: `rgb(${primitives['orange-950']})`
			},
			purple: {
				50: `rgb(${primitives['purple-50']})`,
				100: `rgb(${primitives['purple-100']})`,
				200: `rgb(${primitives['purple-200']})`,
				300: `rgb(${primitives['purple-300']})`,
				400: `rgb(${primitives['purple-400']})`,
				500: `rgb(${primitives['purple-500']})`,
				600: `rgb(${primitives['purple-600']})`,
				700: `rgb(${primitives['purple-700']})`,
				800: `rgb(${primitives['purple-800']})`,
				900: `rgb(${primitives['purple-900']})`,
				950: `rgb(${primitives['purple-950']})`
			},
			pink: {
				50: `rgb(${primitives['pink-50']})`,
				100: `rgb(${primitives['pink-100']})`,
				200: `rgb(${primitives['pink-200']})`,
				300: `rgb(${primitives['pink-300']})`,
				400: `rgb(${primitives['pink-400']})`,
				500: `rgb(${primitives['pink-500']})`,
				600: `rgb(${primitives['pink-600']})`,
				700: `rgb(${primitives['pink-700']})`,
				800: `rgb(${primitives['pink-800']})`,
				900: `rgb(${primitives['pink-900']})`,
				950: `rgb(${primitives['pink-950']})`
			},
			lime: {
				50: `rgb(${primitives['lime-50']})`,
				100: `rgb(${primitives['lime-100']})`,
				200: `rgb(${primitives['lime-200']})`,
				300: `rgb(${primitives['lime-300']})`,
				400: `rgb(${primitives['lime-400']})`,
				500: `rgb(${primitives['lime-500']})`,
				600: `rgb(${primitives['lime-600']})`,
				700: `rgb(${primitives['lime-700']})`,
				800: `rgb(${primitives['lime-800']})`,
				900: `rgb(${primitives['lime-900']})`,
				950: `rgb(${primitives['lime-950']})`
			},
			yellow: {
				50: `rgb(${primitives['yellow-50']})`,
				100: `rgb(${primitives['yellow-100']})`,
				200: `rgb(${primitives['yellow-200']})`,
				300: `rgb(${primitives['yellow-300']})`,
				400: `rgb(${primitives['yellow-400']})`,
				500: `rgb(${primitives['yellow-500']})`,
				600: `rgb(${primitives['yellow-600']})`,
				700: `rgb(${primitives['yellow-700']})`,
				800: `rgb(${primitives['yellow-800']})`,
				900: `rgb(${primitives['yellow-900']})`,
				950: `rgb(${primitives['yellow-950']})`
			},
			slate: {
				50: '#f8fafc',
				100: '#f1f5f9',
				200: '#e2e8f0',
				300: '#cbd5e1',
				400: '#94a3b8',
				500: '#64748b',
				600: '#475569',
				700: '#334155',
				800: '#1e293b',
				900: '#0f172a'
			},
			zinc: {
				50: '#fafafa',
				100: '#f4f4f5',
				200: '#e4e4e7',
				300: '#d4d4d8',
				400: '#a1a1aa',
				500: '#71717a',
				600: '#52525b',
				700: '#3f3f46',
				800: '#27272a',
				900: '#18181b'
			},
			neutral: {
				50: '#fafafa',
				100: '#f5f5f5',
				200: '#e5e5e5',
				300: '#d4d4d4',
				400: '#a3a3a3',
				500: '#737373',
				600: '#525252',
				700: '#404040',
				800: '#262626',
				900: '#171717'
			},
			stone: {
				50: '#fafaf9',
				100: '#f5f5f4',
				200: '#e7e5e4',
				300: '#d6d3d1',
				400: '#a8a29e',
				500: '#78716c',
				600: '#57534e',
				700: '#44403c',
				800: '#292524',
				900: '#1c1917'
			},
			amber: {
				50: '#fffbeb',
				100: '#fef3c7',
				200: '#fde68a',
				300: '#fcd34d',
				400: '#fbbf24',
				500: '#f59e0b',
				600: '#d97706',
				700: '#b45309',
				800: '#92400e',
				900: '#78350f'
			},
			emerald: {
				50: '#ecfdf5',
				100: '#d1fae5',
				200: '#a7f3d0',
				300: '#6ee7b7',
				400: '#34d399',
				500: '#10b981',
				600: '#059669',
				700: '#047857',
				800: '#065f46',
				900: '#064e3b'
			},
			teal: {
				50: '#f0fdfa',
				100: '#ccfbf1',
				200: '#99f6e4',
				300: '#5eead4',
				400: '#2dd4bf',
				500: '#14b8a6',
				600: '#0d9488',
				700: '#0f766e',
				800: '#115e59',
				900: '#134e4a'
			},
			cyan: {
				50: '#ecfeff',
				100: '#cffafe',
				200: '#a5f3fc',
				300: '#67e8f9',
				400: '#22d3ee',
				500: '#06b6d4',
				600: '#0891b2',
				700: '#0e7490',
				800: '#155e75',
				900: '#164e63'
			},
			sky: {
				50: '#f0f9ff',
				100: '#e0f2fe',
				200: '#bae6fd',
				300: '#7dd3fc',
				400: '#38bdf8',
				500: '#0ea5e9',
				600: '#0284c7',
				700: '#0369a1',
				800: '#075985',
				900: '#0c4a6e'
			},
			violet: {
				50: '#f5f3ff',
				100: '#ede9fe',
				200: '#ddd6fe',
				300: '#c4b5fd',
				400: '#a78bfa',
				500: '#8b5cf6',
				600: '#7c3aed',
				700: '#6d28d9',
				800: '#5b21b6',
				900: '#4c1d95'
			},
			fuchsia: {
				50: '#fdf4ff',
				100: '#fae8ff',
				200: '#f5d0fe',
				300: '#f0abfc',
				400: '#e879f9',
				500: '#d946ef',
				600: '#c026d3',
				700: '#a21caf',
				800: '#86198f',
				900: '#701a75'
			},
			rose: {
				50: '#fff1f2',
				100: '#ffe4e6',
				200: '#fecdd3',
				300: '#fda4af',
				400: '#fb7185',
				500: '#f43f5e',
				600: '#e11d48',
				700: '#be123c',
				800: '#9f1239',
				900: '#881337'
			},
			gray: {
				50: '#f9fafb',
				100: '#f3f4f6',
				200: '#e5e7eb',
				300: '#d1d5db',
				400: '#9ca3af',
				500: '#6b7280',
				600: '#4b5563',
				700: '#374151',
				800: '#1f2937',
				900: '#111827'
			},
			indigo: {
				50: '#eef2ff',
				100: '#e0e7ff',
				200: '#c7d2fe',
				300: '#a5b4fc',
				400: '#818cf8',
				500: '#6366f1',
				600: '#4f46e5',
				700: '#4338ca',
				800: '#3730a3',
				900: '#312e81'
			},
			frost: {
				50: '#f5f7fa',
				100: '#eaeef4',
				200: '#cfd9e8',
				300: '#a6bad3',
				400: '#7594bb',
				500: '#5e81ac',
				600: '#415f88',
				700: '#364d6e',
				800: '#2f425d',
				900: '#2b394f',
				950: '#1d2534'
			},
			marine: {
				50: '#E9E9F4',
				100: '#859AC7',
				200: '#586F9E',
				300: '#4A5F8A',
				400: '#394A6D',
				500: '#323F5B'
			},
			nord: {
				0: '#2E3440',
				100: '#3B4252',
				200: '#434C5E',
				300: '#4C566A',
				400: '#D8DEE9',
				500: '#E5E9F0',
				600: '#ECEFF4',
				700: '#8FBCBB',
				800: '#88C0D0',
				900: '#81A1C1',
				950: '#5E81AC'
			}
		},
		fontFamily: {
			// add double quotes if there is space in font name
			main: ['Inter', 'sans-serif'],
			mono: [
				'ui-monospace',
				'SFMono-Regular',
				'Menlo',
				'Monaco',
				'Consolas',
				'"Liberation Mono"',
				'"Courier New"',
				'monospace'
			]
		},
		extend: {
			border: {
				color: 'red'
			},
			borderRadius: {
				component: '0.250rem'
			},
			maxHeight: {
				'1/2': '50vh',
				'2/3': '66vh',
				'3/4': '75vh'
			},
			minWidth: {
				'1/4': '25%',
				'1/3': '33%',
				'1/2': '50%',
				'2/3': '66%',
				'3/4': '75%'
			},
			minHeight: {
				'1/2': '50vh'
			},
			height: {
				'2/3': '66vh'
			},
			transitionProperty: {
				height: 'height'
			},
			fontSize: {
				'2xs': '0.7rem',
				'3xs': '0.65rem'
			},
			screens: {
				fhd: '1900px',
				qhd: '2500px',
				'4k': '3800px'
			},
			dropShadow: {
				base: ['0 2px 1px rgba(0, 0, 0, 0.06)', '0 2px 2px rgba(0, 0, 0, 0.1)']
			},
			animation: {
				'spin-counter-clockwise': 'spin-counter-clockwise 1s linear infinite',
				'zoom-in': 'zoom-in 0.25s ease-in-out',
				'fade-out': 'fade-out 1s ease-in-out'
			},
			keyframes: {
				'spin-counter-clockwise': {
					to: { transform: 'rotate(-360deg)' }
				},
				'zoom-in': {
					'0%': { transform: 'scale(0.95)' },
					'100%': { transform: 'scale(1)' }
				},
				'fade-out': {
					'0%': { opacity: '1' },
					'100%': { opacity: '0' }
				}
			}
		}
	},

	plugins: [
		require('@tailwindcss/forms'),
		require('@tailwindcss/typography'),
		plugin(({ addBase, addComponents, addUtilities, theme }) => {
			let lightColorVariables = Object.fromEntries([
				...Object.entries(tokens.light).map(([key, value]) => [`--color-${key}`, value]),
				...Object.entries(tokens.dark).map(([key, value]) => [`--color-${key}-inverse`, value])
			])
			let darkColorVariables = Object.fromEntries([
				...Object.entries(tokens.dark).map(([key, value]) => [`--color-${key}`, value]),
				...Object.entries(tokens.light).map(([key, value]) => [`--color-${key}-inverse`, value])
			])

			addBase({
				html: {
					fontFamily: theme('fontFamily.main'),
					fontSize: theme('fontSize.base'),
					fontWeight: theme('fontWeight.normal'),

					...lightColorVariables,
					backgroundColor: 'rgb(var(--color-surface-primary))',
					color: 'rgb(var(--color-text-primary))',

					'--vscode-editorSuggestWidget-background': '#f3f3f3',
					'--vscode-editorHoverWidget-foreground': '#616161',
					'--vscode-editorHoverWidget-border': '#c8c8c8',
					'--vscode-editorHoverWidget-statusBarBackground': '#e7e7e7',
					'--vscode-editorSuggestWidget-foreground': '#eeffff',
					'--vscode-editorSuggestWidget-highlightForeground': '#80cbc4',
					'--vscode-editorSuggestWidget-selectedBackground': 'rgba(0, 0, 0, 0.31)',

					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: theme('fontSize.lg')
					},

					'&.dark': {
						backgroundColor: `rgb(${tokens.dark['surface-primary']})`,
						color: `rgb(${tokens.dark['text-primary']})`,

						...darkColorVariables,

						'--vscode-editorSuggestWidget-background': '#252526',
						'--vscode-editorHoverWidget-foreground': '#cccccc',
						'--vscode-editorHoverWidget-border': '#454545',
						'--vscode-editorHoverWidget-statusBarBackground': '#2c2c2d'
					}
				},
				h1: {
					fontSize: '24px',
					fontWeight: theme('fontWeight.extrabold'),
					lineHeight: '1.05',
					[`@media (min-width: ${theme('screens.lg')})`]: {
						fontSize: '26px'
					},
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '29px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '34px'
					}
				},
				h2: {
					fontSize: '20px',
					fontWeight: theme('fontWeight.extrabold'),
					lineHeight: '1.1',
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '22px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '25px'
					}
				},
				mark: {
					backgroundColor: theme('colors.yellow.200'),
					borderRadius: theme('borderRadius.sm')
				},
				h3: {
					fontSize: '18px',
					fontWeight: theme('fontWeight.bold'),
					lineHeight: '1.2',
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '20px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '22px'
					}
				},
				h4: {
					fontSize: '18px',
					fontWeight: theme('fontWeight.semibold'),
					lineHeight: '1.3',
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '20px'
					}
				},
				h5: {
					fontSize: '16px',
					fontWeight: theme('fontWeight.semibold'),
					lineHeight: '1.5',
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '18px'
					}
				},
				h6: {
					fontSize: '16px',
					fontWeight: theme('fontWeight.medium'),
					lineHeight: '1.5',
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '18px'
					}
				},
				button: {
					fontWeight: theme('fontWeight.semibold')
				},
				a: {
					color: 'rgb(var(--color-text-accent))'
				},
				'.dark input::placeholder': {
					color: theme('colors.gray.400')
				},
				"[type='checkbox']:checked": {
					backgroundImage: `url("data:image/svg+xml,%3csvg viewBox='0 0 16 16' fill='black' xmlns='http://www.w3.org/2000/svg'%3e%3cpath d='M12.207 4.793a1 1 0 010 1.414l-5 5a1 1 0 01-1.414 0l-2-2a1 1 0 011.414-1.414L6.5 9.086l4.293-4.293a1 1 0 011.414 0z'/%3e%3c/svg%3e")`
				},
				".dark [type='checkbox']:checked": {
					backgroundImage: `url("data:image/svg+xml,%3csvg viewBox='0 0 16 16' fill='white' xmlns='http://www.w3.org/2000/svg'%3e%3cpath d='M12.207 4.793a1 1 0 010 1.414l-5 5a1 1 0 01-1.414 0l-2-2a1 1 0 011.414-1.414L6.5 9.086l4.293-4.293a1 1 0 011.414 0z'/%3e%3c/svg%3e")`
				},
				'input:not(.windmillapp):not(.no-default-style),input[type="text"]:not(.windmillapp):not(.no-default-style),input[type="email"]:not(.windmillapp):not(.no-default-style),input[type="url"]:not(.windmillapp):not(.no-default-style),input[type="password"]:not(.windmillapp):not(.no-default-style),input[type="number"]:not(.windmillapp):not(.no-default-style),input[type="date"]:not(.windmillapp):not(.no-default-style),input[type="datetime-local"]:not(.windmillapp):not(.no-default-style),input[type="month"]:not(.windmillapp):not(.no-default-style),input[type="search"]:not(.windmillapp):not(.no-default-style),input[type="tel"]:not(.windmillapp):not(.no-default-style),input[type="time"]:not(.windmillapp):not(.no-default-style),input[type="week"]:not(.windmillapp):not(.no-default-style),textarea:not(.windmillapp):not(.no-default-style):not(.monaco-mouse-cursor-text),select:not(.windmillapp):not(.no-default-style)':
					{
						display: 'block',
						fontSize: theme('fontSize.xs'),
						width: '100%',
						padding: `${theme('spacing.1')} ${theme('spacing.2')}`,
						backgroundColor: 'rgb(var(--color-surface-input))' + ' !important',
						border: '1px solid rgb(var(--color-border-light))',
						borderRadius: theme('borderRadius.md'),
						'&:hover': {
							borderColor: 'rgb(var(--color-border-selected) / 0.5)'
						},
						'&:focus': {
							borderColor: 'rgb(var(--color-border-selected))',
							outline: 'none',
							boxShadow: 'none'
						},
						'&:disabled,[disabled]': {
							backgroundColor: 'rgba(var(--color-surface-disabled) / 0.2)' + ' !important'
						}
					},

				'button:disabled,button[disabled=true],a:disabled,a[disabled=true]': {
					pointerEvents: 'none',
					cursor: 'default',
					opacity: '0.90'
				},
				'pre code.hljs': {
					padding: '0px !important',
					fontFamily: theme('fontFamily.mono'),
					fontSize: theme('fontSize.xs') + ' !important',
					lineHeight: theme('lineHeight.4') + ' !important'
				},
				'.h1-textarea': {
					fontSize: '24px !important',
					fontWeight: `${theme('fontWeight.extrabold')} !important`,
					lineHeight: '1.05 !important',
					color: theme('!colors.gray.800'),
					[`@media (min-width: ${theme('screens.lg')})`]: {
						fontSize: '26px !important'
					},
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '29px !important'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '34px !important'
					}
				},
				'.h3-textarea': {
					fontSize: '18px !important',
					fontWeight: `${theme('fontWeight.bold')} !important`,
					lineHeight: '1.2 !important',
					color: theme('!colors.gray.600'),
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '20px !important'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '22px !important'
					}
				},
				'.p-textarea': {
					fontSize: '16px !important',
					fontWeight: `${theme('fontWeight.normal')} !important`,
					lineHeight: '1.5 !important',
					color: theme('!colors.gray.600'),
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '18px !important'
					},

					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '20px !important'
					}
				},
				input: {
					backgroundColor: 'rgb(var(--color-surface-secondary) / 30%) !important'
				},
				textarea: {
					backgroundColor: 'rgb(var(--color-surface-secondary) / 30%) !important'
				}
			})
			addComponents({
				'.table-custom': {
					'& th': {
						paddingTop: theme('spacing.2'),
						paddingRight: theme('spacing.1'),
						paddingLeft: theme('spacing.1'),
						paddingBottom: theme('spacing.2'),
						fontSize: theme('fontSize.2xs'),
						textAlign: 'left',
						fontWeight: theme('fontWeight.normal'),
						color: 'rgb(var(--color-text-secondary))',
						textTransform: 'none'
					},
					'.dark & th': {
						color: 'rgb(var(--color-text-secondary))'
					},
					'& td': {
						paddingLeft: theme('spacing.1'),
						paddingRight: theme('spacing.1'),
						paddingTop: theme('spacing.2'),
						paddingBottom: theme('spacing.2'),
						fontSize: theme('fontSize.xs'),
						color: 'rgb(var(--color-text-primary))'
					},
					'.dark & td ': {
						color: 'rgb(var(--color-text-primary))'
					},
					'& tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid rgb(var(--color-border-light))`
					},
					'.dark & tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid rgb(var(--color-border-light))`
					},
					'& tbody > tr:hover': {
						backgroundColor: 'rgb(var(--color-surface-hover))'
					},
					'.dark & tbody > tr:hover': {
						backgroundColor: 'rgb(var(--color-surface-hover))'
					}
				},
				'.commit-hash': {
					fontSize: theme('fontSize.2xs'),
					color: theme('colors.gray.500'),
					backgroundColor: theme('colors.gray.200'),
					fontFamily: theme('fontFamily.mono')
				},
				'.input-error': {
					borderColor: `${theme('colors.red.500')} !important`
				},
				'.box': {
					borderWidth: '1px',
					borderRadius: theme('borderRadius.sm'),
					boxShadow: theme('boxShadow.sm'),
					padding: theme('spacing.4')
				},
				'.animate-skeleton': {
					animation: theme('animation.pulse'),
					backgroundColor: theme('colors.blue.100'),
					borderRadius: theme('borderRadius.DEFAULT')
				},
				'.text-black-gradient': {
					color: 'transparent',
					backgroundClip: 'text',
					backgroundImage: `linear-gradient(to right, ${theme('colors.black')}, ${theme(
						'colors.gray.600'
					)})`
				},
				'.splitpanes__pane': {
					backgroundColor: `rgb(${tokens.light['surface-primary']})` + ' !important',
					overflow: 'auto !important'
				},
				'.dark .splitpanes__pane': {
					backgroundColor: `rgb(${tokens.dark['surface-primary']})` + ' !important',
					overflow: 'auto !important'
				},
				'.splitpanes__splitter': {
					backgroundColor: `rgb(${tokens.light['border-light']})` + ' !important',
					margin: '0 !important',
					border: 'none !important',
					'&::after': {
						backgroundColor: `rgb(${tokens.light['border-light']})` + ' !important',
						margin: '0 !important',
						transform: 'none !important',
						transition: 'opacity 200ms !important',
						opacity: '0',
						'--splitter-hover-size': '5px',
						'--splitter-hover-adjustment': '-2px'
					},
					'&:hover::after': {
						opacity: '1',
						zIndex: '1001 !important'
					}
				},
				'.dark .splitpanes__splitter': {
					backgroundColor: `rgb(${tokens.dark['border-light']})` + ' !important',
					'&::after': {
						backgroundColor: `rgb(${tokens.dark['border-light']})` + ' !important'
					}
				},
				'.splitpanes--vertical>.splitpanes__splitter': {
					width: '1px !important',
					'&::before': {
						left: '1px !important',
						width: '0px !important',
						marginLeft: '0 !important'
					},
					'&::after': {
						top: '0 !important',
						height: '100% !important',
						left: 'var(--splitter-hover-adjustment) !important',
						width: 'var(--splitter-hover-size) !important'
					}
				},
				'.splitpanes--horizontal>.splitpanes__splitter': {
					height: '1px !important',
					'&::before': {
						top: '1px !important',
						height: '0px !important',
						marginTop: '0 !important'
					},
					'&::after': {
						top: 'var(--splitter-hover-adjustment) !important',
						height: 'var(--splitter-hover-size) !important',
						left: '0 !important',
						width: '100% !important'
					}
				},
				// Windmill Tab classes

				'.wm-tab-active': {
					borderColor: `rgb(${tokens.dark['border-light']})`,
					color: `rgb(${tokens.light['text-primary']})`
				},

				'.dark .wm-tab-active': {
					borderColor: `rgb(${tokens.dark['border-light']})`,
					color: `rgb(${tokens.dark['text-primary']})`
				}
			})

			addUtilities({
				'.separator': {
					backgroundColor: `${`rgb(${tokens.dark['border-light']})`} !important`
				},
				'.dark .separator': {
					backgroundColor: `${`rgb(${tokens.dark['border-light']})`} !important`
				},
				'.center-center': {
					display: 'flex',
					justifyContent: 'center',
					alignItems: 'center'
				},
				'.inner-border': {
					boxShadow: `inset 0 0 0 1px ${`rgb(${tokens.dark['border-light']})`}`
				},
				'.dark .inner-border': {
					boxShadow: `inset 0 0 0 1px ${`rgb(${tokens.dark['border-light']})`}`
				},
				'.z5000': {
					zIndex: '5000 !important'
				},
				'.ellipsize': {
					overflow: 'hidden',
					whiteSpace: 'nowrap',
					textOverflow: 'ellipsis'
				},
				/** Set the '-webkit-line-clamp' property to the desired number of lines.
				 *
				 * Eg.: `class="ellipsize-multi-line [-webkit-line-clamp:3]"`
				 */
				'.ellipsize-multi-line': {
					display: '-webkit-box',
					'-webkit-box-orient': 'vertical',
					overflow: 'hidden'
				},
				'.disabled': {
					pointerEvents: 'none',
					cursor: 'default',
					filter: 'grayscale(1)'
				},
				'.scrollbar-hidden': {
					'-ms-overflow-style': 'none',
					'scrollbar-width': 'none',
					'&::-webkit-scrollbar': {
						display: 'none',
						width: '0px'
					}
				}
			})
		})
	],
	darkMode: 'class'
}

module.exports = config

/**
 * @template T
 * @param {T} obj
 * @returns {T}
 */
function makeRgb(obj) {
	const result = {}
	for (const [key, value] of Object.entries(obj)) {
		if (typeof value === 'object' && value !== null) {
			result[key] = makeRgb(value)
		} else if (typeof value === 'string' && value.startsWith('#')) {
			result[key] = hexToRgb(value)
		} else {
			result[key] = value
		}
	}
	return result
}

function hexToRgb(hex) {
	// Remove '#' symbol from the beginning of the hex value
	hex = hex.replace('#', '')

	// Convert the hex value to decimal
	const r = parseInt(hex.substring(0, 2), 16)
	const g = parseInt(hex.substring(2, 4), 16)
	const b = parseInt(hex.substring(4, 6), 16)

	// Check if hex has alpha channel (8 characters)
	if (hex.length === 8) {
		const a = parseInt(hex.substring(6, 8), 16) / 255
		// Return RGB with alpha as decimal value for CSS
		return `${r} ${g} ${b} / ${a.toFixed(3)}`
	}

	// Return the RGB string format without alpha
	return `${r} ${g} ${b}`
}
