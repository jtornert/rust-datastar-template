import { load } from "/assets/js/datastar.js";

load({
    type: "attribute",
    name: "cloak",
    onLoad(ctx) {
        const { el } = ctx;

        el.removeAttribute("data-cloak");
    },
});
