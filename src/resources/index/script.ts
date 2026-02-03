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
