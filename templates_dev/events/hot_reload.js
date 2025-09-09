(() => {
    const element = document.querySelector("[href*='{{ href }}']");

    if (element) {
        const other = element.cloneNode();
        other.setAttribute("href", new URL(element.href).pathname + "?t=" + new Date().valueOf());
        element.remove();
        document.head.appendChild(other);
    } else {
        window.location.reload();
    }
})();
