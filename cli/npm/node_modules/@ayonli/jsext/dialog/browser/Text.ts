export default function Text(message: string) {
    const text = document.createElement("p");
    text.innerHTML = message.replace(/ /g, "&nbsp;").replace(/\n/g, "<br />");
    text.style.margin = "0 0 1rem";
    text.style.fontSize = "1em";
    text.style.width = "100%";
    text.style.overflowX = "auto";
    return text;
}
