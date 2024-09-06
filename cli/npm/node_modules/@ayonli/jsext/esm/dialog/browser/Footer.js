function Footer(...children) {
    const block = document.createElement("footer");
    block.style.display = "flex";
    block.style.justifyContent = "flex-end";
    block.style.alignItems = "center";
    block.style.gap = "0.5em";
    children.forEach(node => {
        block.appendChild(node);
    });
    return block;
}

export { Footer as default };
//# sourceMappingURL=Footer.js.map
