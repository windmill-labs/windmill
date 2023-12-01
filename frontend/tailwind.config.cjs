const plugin = require('tailwindcss/plugin')

const lightTheme = {
	surface: '#ffffff',
	surfaceSecondary: '#f3f4f6',
	surfaceHover: '#e5e7eb',
	surfaceDisabled: '#f9fafb',
	surfaceSelected: '#d1d5db',

	textPrimary: '#2d3748',
	textSecondary: '#4a5568',
	textTertiary: '#505c70',
	textDisabled: '#a0aec0',

	border: '#ddd',
	borderHover: '#ccc'
}

const lightThemeRgb = makeRgb(lightTheme)

const darkTheme = {
	surface: '#2e3440',
	surfaceSecondary: '#3b4252',
	surfaceHover: '#4c566a',
	surfaceDisabled: '#2a2f3a',
	surfaceSelected: '#434c5e',

	textPrimary: '#f3f6f8',
	textSecondary: '#e0e7ed',
	textTertiary: '#c7ccd6',
	textDisabled: '#a0aec0',

	border: '#3e4c60',
	borderHover: '#3e4c60'
}

const darkThemeRgb = makeRgb(darkTheme)

function makeRgb(theme) {
	return Object.fromEntries(
		Object.entries(theme).map(([key, value]) => {
			if (typeof value === 'string' && value.startsWith('#')) {
				return [key, hexToRgb(value)]
			}

			return [key, value]
		})
	)
}

function hexToRgb(hex) {
	// Remove '#' symbol from the beginning of the hex value
	hex = hex.replace('#', '')

	// Convert the hex value to decimal
	const r = parseInt(hex.substring(0, 2), 16)
	const g = parseInt(hex.substring(2, 4), 16)
	const b = parseInt(hex.substring(4, 6), 16)

	// Return the RGB string format
	return `${r} ${g} ${b}`
}

/** @type {import('tailwindcss').Config} */
const config = {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	safelist: [
		'hljs',
		'splitpanes__pane',
		'splitpanes__splitter',
		'wm-tab',
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
	],
	theme: {
		colors: {
			current: 'currentcolor',
			transparent: 'transparent',
			white: '#ffffff',
			black: '#000000',
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
			orange: {
				50: '#fff7ed',
				100: '#ffedd5',
				200: '#fed7aa',
				300: '#fdba74',
				400: '#fb923c',
				500: '#f97316',
				600: '#ea580c',
				700: '#c2410c',
				800: '#9a3412',
				900: '#7c2d12'
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
			lime: {
				50: '#f7fee7',
				100: '#ecfccb',
				200: '#d9f99d',
				300: '#bef264',
				400: '#a3e635',
				500: '#84cc16',
				600: '#65a30d',
				700: '#4d7c0f',
				800: '#3f6212',
				900: '#365314'
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
			purple: {
				50: '#faf5ff',
				100: '#f3e8ff',
				200: '#e9d5ff',
				300: '#d8b4fe',
				400: '#c084fc',
				500: '#a855f7',
				600: '#9333ea',
				700: '#7e22ce',
				800: '#6b21a8',
				900: '#581c87'
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
			pink: {
				50: '#fdf2f8',
				100: '#fce7f3',
				200: '#fbcfe8',
				300: '#f9a8d4',
				400: '#f472b6',
				500: '#ec4899',
				600: '#db2777',
				700: '#be185d',
				800: '#9d174d',
				900: '#831843'
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
			red: {
				50: '#fef2f2',
				100: '#fee2e2',
				200: '#fecaca',
				300: '#fca5a5',
				400: '#f87171',
				500: '#ef4444',
				600: '#dc2626',
				700: '#b91c1c',
				800: '#991b1b',
				900: '#7f1d1d'
			},
			orange: {
				100: '#ffedd5',
				200: '#ffedd5',
				300: '#FDC089',
				400: '#fb923c',
				500: '#f97316',
				600: '#ea580c',
				700: '#c2410c',
				800: '#c2410c'
			},
			yellow: {
				50: '#fefce8',
				100: '#fef9c3',
				200: '#fef08a',
				300: '#fde047',
				400: '#facc15',
				500: '#eab308',
				600: '#ca8a04',
				700: '#a16207',
				800: '#854d0e',
				900: '#713f12'
			},
			green: {
				50: '#f0fdf4',
				100: '#dcfce7',
				200: '#bbf7d0',
				300: '#86efac',
				400: '#4ade80',
				500: '#22c55e',
				600: '#16a34a',
				700: '#15803d',
				800: '#166534',
				900: '#14532d'
			},
			blue: {
				50: '#eff6ff',
				100: '#dbeafe',
				200: '#bfdbfe',
				300: '#93c5fd',
				400: '#60a5fa',
				500: '#3b82f6',
				600: '#2563eb',
				700: '#1d4ed8',
				800: '#1e40af',
				900: '#1e3a8a'
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
			surface: 'rgb(var(--color-surface) / <alpha-value>)',
			'surface-secondary': 'rgb(var(--color-surface-secondary) / <alpha-value>)',
			'surface-hover': 'rgb(var(--color-surface-hover) / <alpha-value>)',
			'surface-disabled': 'rgb(var(--color-surface-disabled) / <alpha-value>)',
			'surface-selected': 'rgb(var(--color-surface-selected) / <alpha-value>)',

			primary: 'rgb(var(--color-text-primary) / <alpha-value>)',
			secondary: 'rgb(var(--color-text-secondary) / <alpha-value>)',
			tertiary: 'rgb(var(--color-text-tertiary) / <alpha-value>)',
			disabled: 'rgb(var(--color-text-disabled) / <alpha-value>)',

			'surface-inverse': 'rgb(var(--color-surface-inverse) / <alpha-value>)',
			'surface-secondary-inverse': 'rgb(var(--color-surface-secondary-inverse) / <alpha-value>)',
			'surface-hover-inverse': 'rgb(var(--color-surface-hover-inverse) / <alpha-value>)',
			'surface-disabled-inverse': 'rgb(var(--color-surface-disabled-inverse) / <alpha-value>)',
			'surface-selected-inverse': 'rgb(var(--color-surface-selected-inverse) / <alpha-value>)',

			'primary-inverse': 'rgb(var(--color-text-primary-inverse) / <alpha-value>)',
			'secondary-inverse': 'rgb(var(--color-text-secondary-inverse) / <alpha-value>)',
			'tertiary-inverse': 'rgb(var(--color-text-tertiary-inverse) / <alpha-value>)',
			'disabled-inverse': 'rgb(var(--color-text-disabled-inverse) / <alpha-value>)'
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
				'2xs': '0.7rem'
			},
			screens: {
				fhd: '1900px',
				qhd: '2500px',
				'4k': '3800px'
			},
			animation: {
				'spin-counter-clockwise': 'spin-counter-clockwise 1s linear infinite'
			},
			keyframes: {
				'spin-counter-clockwise': {
					to: { transform: 'rotate(-360deg)' }
				}
			}
		}
	},

	plugins: [
		require('@tailwindcss/forms'),
		require('@tailwindcss/typography'),
		plugin(({ addBase, addComponents, addUtilities, theme }) => {
			addBase({
				html: {
					fontFamily: theme('fontFamily.main'),
					fontSize: theme('fontSize.base'),
					fontWeight: theme('fontWeight.normal'),

					backgroundColor: 'rgb(var(--color-surface) / <alpha-value>)',
					color: lightTheme.textPrimary,

					'--color-surface': lightThemeRgb.surface,
					'--color-surface-secondary': lightThemeRgb.surfaceSecondary,
					'--color-surface-hover': lightThemeRgb.surfaceHover,
					'--color-surface-disabled': lightThemeRgb.surfaceDisabled,
					'--color-surface-selected': lightThemeRgb.surfaceSelected,

					'--color-text-primary': lightThemeRgb.textPrimary,
					'--color-text-secondary': lightThemeRgb.textSecondary,
					'--color-text-tertiary': lightThemeRgb.textTertiary,
					'--color-text-disabled': lightThemeRgb.textDisabled,

					'--color-surface-inverse': darkThemeRgb.surface,
					'--color-surface-secondary-inverse': darkThemeRgb.surfaceSecondary,
					'--color-surface-hover-inverse': darkThemeRgb.surfaceHover,
					'--color-surface-disabled-inverse': darkThemeRgb.surfaceDisabled,
					'--color-surface-selected-inverse': darkThemeRgb.surfaceSelected,

					'--color-text-primary-inverse': darkThemeRgb.textPrimary,
					'--color-text-secondary-inverse': darkThemeRgb.textSecondary,
					'--color-text-tertiary-inverse': darkThemeRgb.textTertiary,
					'--color-text-disabled-inverse': darkThemeRgb.textDisabled,

					'--color-border': lightThemeRgb.border,
					'--color-border-hover': lightThemeRgb.borderHover,

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
						backgroundColor: darkTheme.surface,
						color: darkTheme.textPrimary,

						'--color-surface': darkThemeRgb.surface,
						'--color-surface-secondary': darkThemeRgb.surfaceSecondary,
						'--color-surface-hover': darkThemeRgb.surfaceHover,
						'--color-surface-disabled': darkThemeRgb.surfaceDisabled,
						'--color-surface-selected': darkThemeRgb.surfaceSelected,

						'--color-text-primary': darkThemeRgb.textPrimary,
						'--color-text-secondary': darkThemeRgb.textSecondary,
						'--color-text-tertiary': darkThemeRgb.textTertiary,
						'--color-text-disabled': darkThemeRgb.textDisabled,

						'--color-surface-inverse': lightThemeRgb.surface,
						'--color-surface-secondary-inverse': lightThemeRgb.surfaceSecondary,
						'--color-surface-hover-inverse': lightThemeRgb.surfaceHover,
						'--color-surface-disabled-inverse': lightThemeRgb.surfaceDisabled,
						'--color-surface-selected-inverse': lightThemeRgb.surfaceSelected,

						'--color-text-primary-inverse': lightThemeRgb.textPrimary,
						'--color-text-secondary-inverse': lightThemeRgb.textSecondary,
						'--color-text-tertiary-inverse': lightThemeRgb.textTertiary,
						'--color-text-disabled-inverse': lightThemeRgb.textDisabled,

						'--color-border': darkThemeRgb.border,
						'--color-border-hover': darkThemeRgb.borderHover,

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
					color: theme('colors.blue.500')
				},
				'.dark input::placeholder': {
					color: theme('colors.gray.400')
				},
				"[type='checkbox']:checked": {
					backgroundImage: `url("data:image/svg+xml,%3csvg viewBox='0 0 16 16' fill='black' xmlns='http://www.w3.org/2000/svg'%3e%3cpath d='M12.207 4.793a1 1 0 010 1.414l-5 5a1 1 0 01-1.414 0l-2-2a1 1 0 011.414-1.414L6.5 9.086l4.293-4.293a1 1 0 011.414 0z'/%3e%3c/svg%3e");`
				},
				".dark [type='checkbox']:checked": {
					backgroundImage: `url("data:image/svg+xml,%3csvg viewBox='0 0 16 16' fill='white' xmlns='http://www.w3.org/2000/svg'%3e%3cpath d='M12.207 4.793a1 1 0 010 1.414l-5 5a1 1 0 01-1.414 0l-2-2a1 1 0 011.414-1.414L6.5 9.086l4.293-4.293a1 1 0 011.414 0z'/%3e%3c/svg%3e");`
				},
				'input:not(.windmillapp),input[type="text"]:not(.windmillapp),input[type="email"]:not(.windmillapp),input[type="url"]:not(.windmillapp),input[type="password"]:not(.windmillapp),input[type="number"]:not(.windmillapp),input[type="date"]:not(.windmillapp),input[type="datetime-local"]:not(.windmillapp),input[type="month"]:not(.windmillapp),input[type="search"]:not(.windmillapp),input[type="tel"]:not(.windmillapp),input[type="time"]:not(.windmillapp),input[type="week"]:not(.windmillapp),textarea:not(.windmillapp):not(.monaco-mouse-cursor-text),select:not(.windmillapp)':
					{
						display: 'block',
						fontSize: theme('fontSize.sm'),
						width: '100%',
						padding: `${theme('spacing.1')} ${theme('spacing.2')}`,
						border: `1px solid ${theme('colors.gray.300')}`,
						borderRadius: theme('borderRadius.md'),
						'&:focus': {
							'--tw-ring-color': theme('colors.frost.100'),
							'--tw-ring-offset-shadow':
								'var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)',
							'--tw-ring-shadow':
								'var(--tw-ring-inset) 0 0 0 calc(3px + var(--tw-ring-offset-width)) var(--tw-ring-color)',
							boxShadow:
								'var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000)'
						},
						'&:disabled,[disabled]': {
							backgroundColor: theme('colors.gray.100') + ' !important',
							'.dark &': {
								backgroundColor: theme('colors.gray.700') + ' !important'
							}
						}
					},
				'.dark input:not(.windmillapp),.dark input[type="text"]:not(.windmillapp),.dark input[type="email"]:not(.windmillapp),.dark input[type="url"]:not(.windmillapp),.dark input[type="password"]:not(.windmillapp),.dark input[type="number"]:not(.windmillapp),.dark input[type="date"]:not(.windmillapp),.dark input[type="datetime-local"]:not(.windmillapp),.dark input[type="month"]:not(.windmillapp),.dark input[type="search"]:not(.windmillapp),.dark input[type="tel"]:not(.windmillapp),.dark input[type="time"]:not(.windmillapp),.dark input[type="week"]:not(.windmillapp),.dark textarea:not(.windmillapp):not(.monaco-mouse-cursor-text),.dark select:not(.windmillapp)':
					{
						backgroundColor: theme('colors.gray.700'),
						color: theme('colors.gray.200'),
						borderColor: theme('colors.gray.600'),
						'&:focus': {
							'--tw-ring-color': theme('colors.frost.700')
						}
					},

				'button:disabled,button[disabled=true],a:disabled,a[disabled=true]': {
					pointerEvents: 'none',
					cursor: 'default',
					filter: 'grayscale(1)'
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
					backgroundColor: theme('colors.surface-secondary') + ' !important'
				},
				textarea: {
					backgroundColor: theme('colors.surface-secondary') + ' !important'
				}
			})
			addComponents({
				'.table-custom': {
					'& th': {
						paddingTop: theme('spacing.3'),
						paddingRight: theme('spacing.1'),
						paddingLeft: theme('spacing.1'),
						paddingBottom: theme('spacing.3'),
						fontSize: theme('fontSize.sm'),
						textAlign: 'left',
						fontWeight: theme('fontWeight.semibold'),
						color: theme('colors.gray.900'),
						textTransform: 'capitalize'
					},
					'.dark & th': {
						color: theme('colors.gray.200')
					},
					'& td': {
						paddingLeft: theme('spacing.1'),
						paddingRight: theme('spacing.1'),
						paddingTop: theme('spacing.2'),
						paddingBottom: theme('spacing.2'),
						fontSize: theme('fontSize.sm'),
						color: theme('colors.gray.700')
					},
					'.dark & td ': {
						color: theme('colors.gray.200')
					},
					'& tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid ${theme('colors.gray.200')}`
					},
					'.dark & tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid ${theme('colors.gray.700')}`
					},
					'& tbody > tr:hover': {
						backgroundColor: theme('colors.gray.50')
					},
					'.dark & tbody > tr:hover': {
						backgroundColor: theme('colors.gray.800')
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
					backgroundColor: lightTheme.surface + ' !important',
					overflow: 'auto !important'
				},
				'.dark .splitpanes__pane': {
					backgroundColor: darkTheme.surface + ' !important',
					overflow: 'auto !important'
				},
				'.splitpanes__splitter': {
					backgroundColor: lightTheme.border + ' !important',
					margin: '0 !important',
					border: 'none !important',
					'&::after': {
						backgroundColor: lightTheme.border + ' !important',
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
					backgroundColor: darkTheme.border + ' !important',
					'&::after': {
						backgroundColor: darkTheme.border + ' !important'
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
					borderColor: darkTheme.border,
					color: lightTheme.textPrimary
				},

				'.dark .wm-tab-active': {
					borderColor: lightTheme.border,
					color: darkTheme.textPrimary
				}
			})

			addUtilities({
				'.separator': {
					backgroundColor: `${lightTheme.border} !important`
				},
				'.dark .separator': {
					backgroundColor: `${darkTheme.border} !important`
				},
				'.center-center': {
					display: 'flex',
					justifyContent: 'center',
					alignItems: 'center'
				},
				'.inner-border': {
					boxShadow: `inset 0 0 0 1px ${lightTheme.border}`
				},
				'.dark .inner-border': {
					boxShadow: `inset 0 0 0 1px ${darkTheme.border}`
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
