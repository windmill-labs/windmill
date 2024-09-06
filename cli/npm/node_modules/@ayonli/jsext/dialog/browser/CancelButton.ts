import { closeDialog } from "./Dialog.ts";
import { i18n, useColorTheme } from "./util.ts";

const lightBgColor = "#fff";
const darkBgColor = "#222";
const lightHoverBgColor = "#f4f4f4";
const darkHoverBgColor = "#333";

const locale = {
    ar: "إلغاء",
    be: "Адмяніць",
    bg: "Отказ",
    bn: "বাতিল",
    bs: "Otkaži",
    cs: "Storno",
    da: "Annuller",
    de: "Abbrechen",
    el: "Ακύρωση",
    en: "Cancel",
    es: "Cancelar",
    et: "Tühista",
    eu: "Ezeztatu",
    fa: "لغو",
    fi: "Peruuta",
    fr: "Annuler",
    hi: "रद्द करें",
    hu: "Mégsem",
    hy: "Չեղարկել",
    id: "Batal",
    is: "Hætta við",
    it: "Annulla",
    iw: "ביטול",
    ja: "キャンセル",
    ko: "취소",
    lb: "Ofbriechen",
    lo: "ຍົກເລີກ",
    lv: "Atcelt",
    lt: "Atšaukti",
    mk: "Откажи",
    ml: "റദ്ദാക്കുക",
    mi: "Whakakore",
    mn: "Цуцлах",
    my: "ပယ်ဖျက်",
    ne: "रद्द गर्नुहोस्",
    nl: "Annuleren",
    no: "Avbryt",
    pl: "Anuluj",
    pt: "Cancelar",
    ro: "Anulare",
    ru: "Отмена",
    sa: "रद्द",
    sk: "Zrušiť",
    sl: "Prekliči",
    so: "Kaari",
    sq: "Anulo",
    sv: "Avbryt",
    th: "ยกเลิก",
    tl: "Kanselahin",
    tr: "İptal",
    ug: "بىكارلاش",
    uk: "Скасувати",
    vi: "Hủy",
    zh: "取消",
};

export default function CancelButton() {
    const button = document.createElement("button");
    const { theme, onChange } = useColorTheme();

    button.textContent = i18n(locale);
    button.style.minWidth = "80px";
    button.style.height = "32px";
    button.style.boxSizing = "border-box";
    button.style.border = "1px solid #8ebceb";
    button.style.borderRadius = "1.5rem";
    button.style.color = "#006bd6";
    button.style.backgroundColor = theme === "light" ? lightBgColor : darkBgColor;
    button.style.userSelect = "none";
    button.style.fontSize = "1em";
    button.style.fontWeight = "500";

    button.addEventListener("mouseover", () => {
        const { theme } = useColorTheme();
        button.style.backgroundColor = theme === "light" ? lightHoverBgColor : darkHoverBgColor;
    });

    button.addEventListener("mouseout", () => {
        const { theme } = useColorTheme();
        button.style.backgroundColor = theme === "light" ? lightBgColor : darkBgColor;
    });

    button.addEventListener("click", (event) => {
        const dialog = (event.target as HTMLButtonElement)?.closest("dialog")!;
        closeDialog(dialog, "Cancel");
    });

    onChange((theme) => {
        button.style.backgroundColor = theme === "light" ? lightBgColor : darkBgColor;
    });

    return button;
}
