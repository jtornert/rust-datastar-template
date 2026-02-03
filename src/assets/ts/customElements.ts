customElements.define(
    "ui-global-element",
    class extends HTMLElement {
        connectedCallback() {
            this.append("Global scripts enabled!");

            document.addEventListener("datastar-fetch", (e) => {
                if (
                    (e as CustomEvent<{ type: string }>).detail.type ===
                    "datastar-patch-elements"
                ) {
                    this.append("Global scripts enabled!");
                }
            });
        }
    },
);
