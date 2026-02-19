document.querySelector("ui-global-element")?.after(" Page scripts enabled! ");

document.addEventListener("datastar-fetch", (e) => {
    if (
        (e as CustomEvent<{ type: string }>).detail.type ===
        "datastar-patch-elements"
    ) {
        document
            .querySelector("ui-global-element")
            ?.after(" Page scripts enabled! ");
    }
});

const interval = setInterval(() => {
    for (const el of document.querySelectorAll<HTMLElement>(
        "[data-deadline]",
    )) {
        if (new Date(el.dataset.deadline!) < new Date()) {
            el.remove();
        }
    }
}, 1000);
