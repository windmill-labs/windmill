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

/** Shown instead on Windmill's free tier, whose server clamps the output below any
 * workspace setting: pointing at that setting would send the user to a no-op. */
export const FREE_TIER_OUTPUT_TOKEN_LIMIT_MESSAGE =
	"The response was cut off at the free tier's output token limit. Ask it to continue, or add your own API key in the workspace AI settings for a higher limit."
