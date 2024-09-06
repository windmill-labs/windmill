import { closeDialog } from "./Dialog.ts";
import { i18n } from "./util.ts";

const locale = {
    ar: "موافق",
    be: "Добра",
    bg: "ОК",
    bn: "ঠিক আছে",
    bs: "U redu",
    cs: "OK",
    da: "OK",
    de: "OK",
    el: "ΟΚ",
    en: "OK",
    es: "Aceptar",
    et: "OK",
    eu: "Ados",
    fa: "باشه",
    fi: "OK",
    fr: "OK",
    hi: "ठीक",
    hu: "OK",
    hy: "Լավ",
    id: "OK",
    is: "Í lagi",
    it: "OK",
    iw: "אישור",
    ja: "OK",
    ko: "확인",
    lb: "OK",
    lo: "ຕົກລົງ",
    lv: "Labi",
    lt: "Gerai",
    mk: "Добро",
    ml: "ശരി",
    mi: "OK",
    mn: "Зөв",
    my: "အိုး",
    ne: "ठिक",
    nl: "OK",
    no: "OK",
    pl: "OK",
    pt: "OK",
    ro: "OK",
    ru: "OK",
    sa: "ठीक",
    sk: "OK",
    sl: "V redu",
    so: "OK",
    sq: "OK",
    sv: "OK",
    th: "ตกลง",
    tl: "OK",
    tr: "Tamam",
    ug: "جەزملە",
    uk: "OK",
    vi: "OK",
    zh: "确定",
};

export default function OkButton() {
    const button = document.createElement("button");

    button.textContent = i18n(locale);
    button.style.minWidth = "80px";
    button.style.height = "32px";
    button.style.boxSizing = "border-box";
    button.style.borderStyle = "none";
    button.style.borderRadius = "1.5rem";
    button.style.color = "#fff";
    button.style.backgroundColor = "#006bd6";
    button.style.userSelect = "none";
    button.style.fontSize = "1em";
    button.style.fontWeight = "500";

    button.addEventListener("mouseover", () => {
        button.style.backgroundColor = "#0174e6";
    });

    button.addEventListener("mouseout", () => {
        button.style.backgroundColor = "#006bd6";
    });

    button.addEventListener("click", (event) => {
        const dialog = (event.target as HTMLButtonElement)?.closest("dialog")!;
        closeDialog(dialog, "OK");
    });

    return button;
}
