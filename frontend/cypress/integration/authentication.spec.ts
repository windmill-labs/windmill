// TODO: Should correctly handle exceptions
Cypress.on('uncaught:exception', (err, runnable) => {
	// returning false here prevents Cypress from
	// failing the test
	return false
})

describe('Authentication', () => {
	it('can login using email and password', () => {
		cy.visit(`${Cypress.env('baseUrl')}/user/login`)

		cy.get('#showPassword').click()
		cy.get('#email').type('admin@windmill.dev')
		cy.get('#password').type('changeme')
		cy.get('.flex > .default-button').click()

		cy.contains('Select a workspace')
	})

	it('should redirect to login page if user is not logged in', () => {
		cy.visit(`${Cypress.env('baseUrl')}/user/workspaces`)
		cy.url().should('include', '/user/login')
	})
})
