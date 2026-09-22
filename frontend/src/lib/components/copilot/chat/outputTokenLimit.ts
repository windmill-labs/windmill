/** The provider ended the response at the request's output token cap. A turn
 * cut off there must fail rather than end quietly: thinking counts toward the
 * cap, so the stop often lands mid-thought and would read as the model giving
 * up. Failing also keeps the turn's output so far for a follow-up. */
export class OutputTokenLimitError extends Error {
	constructor() {
		super(
			"The response was cut off at the model's output token limit. Ask it to continue, or raise the limit in the workspace AI settings under Model output limits."
		)
		this.name = 'OutputTokenLimitError'
	}
}
