import { load } from "https://cdn.jsdelivr.net/gh/starfederation/datastar@main/bundles/datastar.js";

load({
    type: "attribute",
    name: "cloak",
    onLoad(ctx) {
        const { el } = ctx;

        el.removeAttribute("data-cloak");
    },
});
