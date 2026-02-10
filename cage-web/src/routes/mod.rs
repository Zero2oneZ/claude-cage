pub mod app;
pub mod codie;
pub mod gentlyos;
pub mod health;
pub mod pages;
pub mod sessions;
pub mod surface;
pub mod tier;

/// Check if the request comes from HTMX (has HX-Request header).
pub fn is_htmx(headers: &axum::http::HeaderMap) -> bool {
    headers.get("HX-Request").is_some()
}

/// HTML-escape a string to prevent XSS in hand-built HTML responses.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Wrap fragment HTML in the base page shell for direct URL access.
/// Mirrors base.html but allows embedding pre-rendered fragment content.
pub fn wrap_page(title: &str, content: &str) -> String {
    let base = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>cage-web | __TITLE__</title>
    <link rel="stylesheet" href="/static/style.css">
    <script src="/static/htmx.min.js"></script>
    <script src="/static/sse.js"></script>
</head>
<body>
    <nav class="sidebar">
        <div class="logo">
            <pre class="logo-art"> &#9614;&#9627;&#9611;&#9611;&#9611;&#9620;&#9612;
&#9629;&#9620;&#9611;&#9611;&#9611;&#9611;&#9611;&#9627;&#9624;
  &#9624;&#9624; &#9629;&#9629;</pre>
            <span class="logo-text">cage-web</span>
        </div>
        <ul class="nav-links">
            <li><a href="/" hx-get="/" hx-target="#main" hx-push-url="true">Dashboard</a></li>
            <li><a href="/sessions" hx-get="/sessions" hx-target="#main" hx-push-url="true">Sessions</a></li>
            <li><a href="/tree" hx-get="/tree" hx-target="#main" hx-push-url="true">GentlyOS Tree</a></li>
            <li><a href="/codie" hx-get="/codie" hx-target="#main" hx-push-url="true">CODIE Programs</a></li>
            <li><a href="/tier" hx-get="/tier" hx-target="#main" hx-push-url="true">Tier Hierarchy</a></li>
            <li><a href="/surface" hx-get="/surface" hx-target="#main" hx-push-url="true">IO Surface</a></li>
        </ul>
        <div class="health-bar" hx-get="/partials/health" hx-trigger="load, every 5s" hx-swap="innerHTML">
            Loading...
        </div>
    </nav>
    <main id="main">
__CONTENT__
    </main>
    <div id="spinner" class="htmx-indicator">Loading...</div>
</body>
</html>"##;
    base.replace("__TITLE__", &html_escape(title))
        .replace("__CONTENT__", content)
}
