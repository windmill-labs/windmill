export type ColorTheme = "light" | "dark";

export function useColorTheme(): {
    theme: ColorTheme;
    onChange: (callback: (theme: "light" | "dark") => void) => void;
} {
    if (typeof window.matchMedia === "function") {
        const result = window.matchMedia("(prefers-color-scheme: dark)");
        const theme = result.matches ? "dark" : "light";
        return {
            theme,
            onChange: (callback) => {
                result.addEventListener("change", () => {
                    callback(result.matches ? "dark" : "light");
                });
            }
        };
    } else {
        return {
            theme: "light",
            onChange: (_) => {
                void 0;
            }
        };
    }
}

export function i18n(locale: { [lang: string]: string; }): string {
    let lang = "en";

    if (typeof navigator === "object" && typeof navigator.language === "string") {
        lang = navigator.language.split("-")[0]!;
    }

    return locale[lang] || locale["en"]!;
}
