const config = {
	content: ['./src/**/*.{html,js,svelte,ts}'],

	theme: {
		extend: {
			maxHeight: {
				'1/2': '50vh',
				'2/3': '66vh',
				'3/4': '75vh',
			},
			minWidth: {
				'1/4': '25%',
				'1/3': '33%',
				'1/2': '50%',
				'2/3': '66%',
			},
			minHeight: {
				'1/2': '50vh',
			}
		},
	},

	plugins: [require('@tailwindcss/forms'), require('@tailwindcss/typography')]
};

module.exports = config;
