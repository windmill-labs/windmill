Cypress.Commands.add('login', (email: string, password: string) => {
	cy.visit(`${Cypress.env('baseUrl')}/user/login`)
	cy.get('#showPassword').click()
	cy.get('#email').type('admin@windmill.dev')
	cy.get('#password').type('changeme')
	cy.get('.flex > .default-button').click()
})
