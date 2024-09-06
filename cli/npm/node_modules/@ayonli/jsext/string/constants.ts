export const EMOJI_CHAR = (() => {
    try {
        return new RegExp("^\(?:\\p{Emoji_Modifier_Base}\\p{Emoji_Modifier}?|\\p{Emoji_Presentation}|\\p{Emoji}\\uFE0F)(?:\\u200d(?:\\p{Emoji_Modifier_Base}\\p{Emoji_Modifier}?|\\p{Emoji_Presentation}|\\p{Emoji}\\uFE0F))*$", "u");
    } catch {
        return new RegExp("^(\\u00a9|\\u00ae|[\\u25a0-\\u27bf]|\\ud83c[\\ud000-\\udfff]|\\ud83d[\\ud000-\\udfff]|\\ud83e[\\ud000-\\udfff])$");
    }
})();
