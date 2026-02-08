// SSE helper for live log streaming
// Used with hx-ext="sse" for real-time container log updates

document.addEventListener('htmx:sseMessage', function(evt) {
    var target = evt.detail.elt;
    if (target && target.classList.contains('log-output')) {
        target.scrollTop = target.scrollHeight;
    }
});

// Auto-scroll log containers on new content
document.addEventListener('htmx:afterSwap', function(evt) {
    var logs = evt.detail.target.querySelectorAll('.log-output');
    logs.forEach(function(el) {
        el.scrollTop = el.scrollHeight;
    });
});
