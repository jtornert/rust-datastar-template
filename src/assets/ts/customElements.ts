customElements.define(
    "ui-global-element",
    class extends HTMLElement {
        connectedCallback() {
            this.append("Global scripts enabled!");
        }
    },
);
