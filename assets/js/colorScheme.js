var setAutoDocumentColorScheme = (isDarkMode) => {
    document.documentElement.dataset.colorScheme = isDarkMode
        ? "dark"
        : "light";
};
function setColorSchemePreference(preference) {
    if (preference === "auto") {
        localStorage.removeItem("colorScheme");
        const isDarkMode = matchMedia("(prefers-color-scheme: dark)").matches;
        setAutoDocumentColorScheme(isDarkMode);
    } else {
        localStorage.setItem("colorScheme", preference);
        document.documentElement.dataset.colorScheme = preference;
    }
}
function colorSchemeSwitcher(element) {
    element.value = localStorage.getItem("colorScheme") ?? "auto";
    element.addEventListener("change", (e) =>
        setColorSchemePreference(e.currentTarget.value)
    );
}
(() => {
    const systemColorScheme = matchMedia("(prefers-color-scheme: dark)");
    const preference = localStorage.getItem("colorScheme");
    if (preference === null)
        setAutoDocumentColorScheme(systemColorScheme.matches);
    else document.documentElement.dataset.colorScheme = preference;
    systemColorScheme.addEventListener("change", (e) => {
        const preference = localStorage.getItem("colorScheme");
        if (preference === null) setAutoDocumentColorScheme(e.matches);
    });
})();
