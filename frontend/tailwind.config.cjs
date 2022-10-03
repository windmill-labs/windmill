const plugin = require('tailwindcss/plugin');

/** @type {import('tailwindcss').Config} */
const config = {
	content: [
		'./src/**/*.{html,js,svelte,ts}',
		"./node_modules/flowbite-svelte/**/*.{html,js,svelte,ts}",
	],
	safelist: [
		'inline-highlight'
	],
	theme: {
		colors: {
			current: 'currentcolor',
			transparent: 'transparent',
			white: '#ffffff',
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
				900: '#111827',
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
				900: '#7f1d1d',
			},
			orange: {
				100: '#ffedd5',
				400: '#fb923c',
				500: '#f97316',
				600: '#ea580c',
				700: '#c2410c',
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
				900: '#713f12',
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
				900: '#14532d',
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
				900: '#1e3a8a',
			},
			indigo: {
				100: '#e0e7ff',
				200: '#c7d2fe',
				300: '#a5b4fc',
				500: '#6366f1',
				800: '#3730a3',
				900: '#312e81',
			}
		},
		extend: {
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
				'height': 'height'
			},
			fontSize: {
				'2xs': '0.7rem'
			}
		}
	},

	plugins: [
		require('@tailwindcss/forms'),
		require('@tailwindcss/typography'),
		require('flowbite/plugin'),
		plugin(({ addBase, addComponents, addUtilities, theme }) => {
			addBase({
				'html': {
					overflowY: 'auto'
				},
				'h1': {
					fontSize: theme('fontSize.2xl'),
					color: theme('colors.gray.700')
				},
				'h1': {
					fontSize: theme('fontSize.xl'),
					color: theme('colors.blue.500')
				},
				'a': {
					color: theme('colors.blue.500')
				},
				'input,input[type="text"],input[type="email"],input[type="url"],input[type="password"],input[type="number"],input[type="date"],input[type="datetime-local"],input[type="month"],input[type="search"],input[type="tel"],input[type="time"],input[type="week"],textarea,select': {
					display: 'block',
					fontSize: theme('fontSize.sm'),
					width: '100%',
					padding: `${theme('spacing.1')} ${theme('spacing.2')}`,
					border: `1px solid ${theme('colors.gray.300')}`,
					borderRadius: theme('borderRadius.md'),
					boxShadow: theme('boxShadow.sm'),
					'&:focus': {
						'--tw-ring-color': theme('colors.indigo.100'),
						'--tw-ring-offset-shadow': 'var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)',
						'--tw-ring-shadow': 'var(--tw-ring-inset) 0 0 0 calc(3px + var(--tw-ring-offset-width)) var(--tw-ring-color)',
						boxShadow: 'var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000)'
					},
					'&:disabled,[disabled]': {
						backgroundColor: theme('colors.gray.100') + ' !important'
					}
				},
				'button:disabled,button[disabled=true],a:disabled,a[disabled=true]': {
					pointerEvents: 'none',
					cursor: 'default',
					filter: 'grayscale(1)'
				}
			});
			addComponents({
				'#table-custom': {
					'& th': {
						paddingTop: theme('spacing.3'),
						paddingBottom: theme('spacing.3'),
						fontSize: theme('fontSize.sm'),
						textAlign: 'left',
						fontWeight: theme('fontWeight.semibold'),
						color: theme('colors.gray.900'),
						textTransform: 'capitalize',
					},
					'& td': {
						paddingTop: theme('spacing.4'),
						paddingBottom: theme('spacing.4'),
						fontSize: theme('fontSize.sm'),
						color: theme('colors.gray.700'),
					},
					'& tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid ${theme('colors.gray.200')}`
					},
				},
				'.commit-hash': {
					fontSize: theme('fontSize.2xs'),
					color: theme('colors.gray.500'),
					backgroundColor: theme('colors.gray.200'),
          fontFamily: theme('fontFamily.mono'),
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
				'.inline-highlight': {
					'& pre code.hljs': {
						padding: '0px',
						fontSize: theme('fontSize.xs'),
						lineHeight: theme('lineHeight.4'),
						whiteSpace: 'pre-wrap'
					}
				}
			});
			addUtilities({
				'.separator': {
					backgroundColor: '#ddd !important'
				},
				'.center-center': {
					display: 'flex',
					justifyContent: 'center',
					alignItems: 'center',
				},
				'.ellipsize': {
					overflow: 'hidden',
					whiteSpace: 'nowrap',
					textOverflow: 'ellipsis',
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
						width: '0px',
					},
				},
			})
		})
	],
	darkMode: 'class',
}

module.exports = config
