//! E-Commerce — full-stack storefront with cart, checkout, orders.
//! Pure Rust, server-rendered HTML, in-memory state, zero JS frameworks.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct Product {
    id: u32,
    name: String,
    description: String,
    price: f64,
    category: String,
    image_emoji: String,
    stock: u32,
    rating: f32,
    reviews: u32,
    featured: bool,
}

#[derive(Clone)]
struct CartItem {
    product_id: u32,
    quantity: u32,
}

#[derive(Clone)]
struct Order {
    id: u32,
    items: Vec<(String, u32, f64)>, // name, qty, price
    total: f64,
    status: String,
    created_at: String,
}

struct AppState {
    products: RwLock<Vec<Product>>,
    carts: RwLock<HashMap<String, Vec<CartItem>>>,
    orders: RwLock<Vec<Order>>,
    next_order_id: RwLock<u32>,
}

fn seed_products() -> Vec<Product> {
    vec![
        Product { id: 1, name: "Mechanical Keyboard".into(), description: "Cherry MX Brown switches, RGB backlit, hot-swappable".into(), price: 149.99, category: "Electronics".into(), image_emoji: "⌨️".into(), stock: 45, rating: 4.7, reviews: 328, featured: true },
        Product { id: 2, name: "Ultra-Wide Monitor".into(), description: "34\" curved IPS, 3440x1440, 144Hz, USB-C hub".into(), price: 599.99, category: "Electronics".into(), image_emoji: "🖥️".into(), stock: 12, rating: 4.9, reviews: 156, featured: true },
        Product { id: 3, name: "Ergonomic Chair".into(), description: "Mesh back, lumbar support, adjustable armrests".into(), price: 449.99, category: "Furniture".into(), image_emoji: "🪑".into(), stock: 23, rating: 4.5, reviews: 89, featured: false },
        Product { id: 4, name: "Noise Cancelling Headphones".into(), description: "ANC, 30hr battery, spatial audio, multipoint".into(), price: 279.99, category: "Audio".into(), image_emoji: "🎧".into(), stock: 67, rating: 4.8, reviews: 512, featured: true },
        Product { id: 5, name: "Standing Desk".into(), description: "Electric sit-stand, 60x30\", memory presets, cable tray".into(), price: 699.99, category: "Furniture".into(), image_emoji: "🗄️".into(), stock: 8, rating: 4.6, reviews: 203, featured: false },
        Product { id: 6, name: "Webcam 4K".into(), description: "4K30/1080p60, auto-framing, noise-cancelling mic".into(), price: 129.99, category: "Electronics".into(), image_emoji: "📷".into(), stock: 34, rating: 4.3, reviews: 178, featured: false },
        Product { id: 7, name: "USB-C Dock".into(), description: "Triple display, 100W PD, 10Gbps, SD card reader".into(), price: 189.99, category: "Electronics".into(), image_emoji: "🔌".into(), stock: 56, rating: 4.4, reviews: 92, featured: false },
        Product { id: 8, name: "Desk Lamp".into(), description: "LED bar, 5 color temps, auto-dimming, USB charge".into(), price: 79.99, category: "Lighting".into(), image_emoji: "💡".into(), stock: 89, rating: 4.2, reviews: 67, featured: false },
        Product { id: 9, name: "Wireless Mouse".into(), description: "Ergonomic vertical, 4000 DPI, dual-mode BT/2.4G".into(), price: 49.99, category: "Electronics".into(), image_emoji: "🖱️".into(), stock: 120, rating: 4.1, reviews: 234, featured: false },
        Product { id: 10, name: "Laptop Stand".into(), description: "Aluminum, adjustable angle, ventilated, foldable".into(), price: 39.99, category: "Accessories".into(), image_emoji: "💻".into(), stock: 200, rating: 4.0, reviews: 145, featured: false },
        Product { id: 11, name: "Microphone Kit".into(), description: "Condenser USB, boom arm, pop filter, shock mount".into(), price: 159.99, category: "Audio".into(), image_emoji: "🎙️".into(), stock: 28, rating: 4.6, reviews: 301, featured: true },
        Product { id: 12, name: "Cable Management Kit".into(), description: "Raceways, clips, sleeves, velcro ties — 120pc set".into(), price: 24.99, category: "Accessories".into(), image_emoji: "🔗".into(), stock: 300, rating: 4.3, reviews: 89, featured: false },
        Product { id: 13, name: "Smart Power Strip".into(), description: "6 outlets, 4 USB-A, 2 USB-C, surge protection, app control".into(), price: 34.99, category: "Electronics".into(), image_emoji: "🔋".into(), stock: 150, rating: 4.5, reviews: 213, featured: false },
        Product { id: 14, name: "Monitor Light Bar".into(), description: "Asymmetric LED, no screen glare, touch controls, USB powered".into(), price: 59.99, category: "Lighting".into(), image_emoji: "🌟".into(), stock: 75, rating: 4.7, reviews: 178, featured: false },
        Product { id: 15, name: "Mechanical Numpad".into(), description: "Wireless, programmable macros, aluminum frame".into(), price: 69.99, category: "Electronics".into(), image_emoji: "🔢".into(), stock: 42, rating: 4.2, reviews: 56, featured: false },
        Product { id: 16, name: "GPU RTX 5090".into(), description: "32GB GDDR7, PCIe 5.0, ray tracing, DLSS 4".into(), price: 1999.99, category: "Electronics".into(), image_emoji: "🎮".into(), stock: 3, rating: 5.0, reviews: 12, featured: true },
    ]
}

const CSS: &str = r#"
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d1117;color:#c9d1d9;font-family:'Segoe UI',system-ui,sans-serif}
a{color:#58a6ff;text-decoration:none}a:hover{text-decoration:underline}
.nav{background:#161b22;border-bottom:1px solid #30363d;padding:0.75rem 2rem;display:flex;align-items:center;gap:2rem}
.nav h1{font-size:1.2rem;color:#f0f6fc}.nav a{color:#8b949e}.nav a:hover{color:#f0f6fc}
.badge{background:#da3633;color:#fff;border-radius:50%;padding:2px 7px;font-size:0.75rem;margin-left:4px}
.container{max-width:1200px;margin:0 auto;padding:1.5rem}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:1.25rem}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:1.25rem;transition:border-color .2s}
.card:hover{border-color:#58a6ff}
.card .emoji{font-size:3rem;text-align:center;padding:1rem 0}
.card h3{color:#f0f6fc;margin:0.5rem 0 0.25rem}
.card .price{color:#3fb950;font-size:1.25rem;font-weight:700}
.card .desc{color:#8b949e;font-size:0.85rem;margin:0.5rem 0}
.card .meta{color:#6e7681;font-size:0.75rem;display:flex;justify-content:space-between}
.stars{color:#f0c000}.stock-low{color:#da3633}.stock-ok{color:#3fb950}
.btn{background:#238636;color:#fff;border:none;padding:0.5rem 1rem;border-radius:6px;cursor:pointer;font-size:0.9rem}
.btn:hover{background:#2ea043}.btn-danger{background:#da3633}.btn-danger:hover{background:#f85149}
.btn-outline{background:transparent;border:1px solid #30363d;color:#c9d1d9}.btn-outline:hover{border-color:#58a6ff}
input,select{background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:6px;padding:0.5rem;font-size:0.9rem}
.search-bar{display:flex;gap:0.75rem;margin-bottom:1.5rem;flex-wrap:wrap}
.search-bar input{flex:1;min-width:200px}.search-bar select{min-width:150px}
.hero{background:linear-gradient(135deg,#161b22 0%,#0d1117 100%);border:1px solid #30363d;border-radius:12px;padding:2.5rem;margin-bottom:2rem;text-align:center}
.hero h2{color:#f0f6fc;font-size:1.8rem;margin-bottom:0.5rem}
.hero p{color:#8b949e}
table{width:100%;border-collapse:collapse}th,td{padding:0.75rem;text-align:left;border-bottom:1px solid #21262d}
th{color:#8b949e;font-weight:600;font-size:0.85rem}
.total-row{font-weight:700;color:#3fb950;font-size:1.1rem}
.speed{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:0.5rem 1rem;margin-top:1rem;text-align:center;color:#6e7681;font-size:0.8rem}
.speed strong{color:#3fb950}
.featured-badge{background:#1f6feb;color:#fff;font-size:0.7rem;padding:2px 8px;border-radius:4px;margin-left:8px}
.cat-filter{display:flex;gap:0.5rem;margin-bottom:1rem;flex-wrap:wrap}
.cat-filter a{padding:4px 12px;border:1px solid #30363d;border-radius:16px;font-size:0.8rem;color:#8b949e}
.cat-filter a:hover,.cat-filter a.active{border-color:#58a6ff;color:#58a6ff;text-decoration:none}
.order-status{padding:3px 10px;border-radius:12px;font-size:0.8rem}
.status-confirmed{background:#0d419d;color:#58a6ff}.status-shipped{background:#1a4731;color:#3fb950}.status-delivered{background:#1a4731;color:#56d364}
"#;

fn nav(cart_count: usize) -> String {
    format!(
        r#"<nav class="nav"><h1>🛒 RustMart</h1>
        <a href="/">Shop</a>
        <a href="/cart">Cart{badge}</a>
        <a href="/orders">Orders</a>
        <a href="/api/products">API</a></nav>"#,
        badge = if cart_count > 0 { format!(r#" <span class="badge">{cart_count}</span>"#) } else { String::new() }
    )
}

fn speed_bar(us: u128) -> String {
    format!(r#"<div class="speed">Rendered in <strong>{us}μs</strong> — Pure Rust, zero JS frameworks</div>"#)
}

fn page(title: &str, nav_html: &str, body: &str, us: u128) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>{title} — RustMart</title><style>{CSS}</style></head>
        <body>{nav_html}<div class="container">{body}{speed}</div></body></html>"#,
        speed = speed_bar(us)
    ))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    category: Option<String>,
    sort: Option<String>,
    min_price: Option<f64>,
    max_price: Option<f64>,
}

async fn storefront(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Html<String> {
    let start = std::time::Instant::now();
    let products = state.products.read().await;
    let carts = state.carts.read().await;
    let cart_count: usize = carts.get("default").map(|c| c.iter().map(|i| i.quantity as usize).sum()).unwrap_or(0);

    let categories: Vec<String> = {
        let mut cats: Vec<String> = products.iter().map(|p| p.category.clone()).collect();
        cats.sort();
        cats.dedup();
        cats
    };

    let mut filtered: Vec<&Product> = products.iter().filter(|p| {
        let q_match = query.q.as_ref().map(|q| {
            let q = q.to_lowercase();
            p.name.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q)
        }).unwrap_or(true);
        let cat_match = query.category.as_ref().map(|c| c == &p.category).unwrap_or(true);
        let min_match = query.min_price.map(|m| p.price >= m).unwrap_or(true);
        let max_match = query.max_price.map(|m| p.price <= m).unwrap_or(true);
        q_match && cat_match && min_match && max_match
    }).collect();

    match query.sort.as_deref() {
        Some("price_asc") => filtered.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()),
        Some("price_desc") => filtered.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()),
        Some("rating") => filtered.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap()),
        Some("name") => filtered.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => filtered.sort_by(|a, b| b.featured.cmp(&a.featured).then(b.rating.partial_cmp(&a.rating).unwrap())),
    }

    let featured: Vec<&&Product> = filtered.iter().filter(|p| p.featured).collect();

    let mut body = String::new();

    if query.q.is_none() && query.category.is_none() {
        body.push_str(&format!(
            r#"<div class="hero"><h2>Welcome to RustMart</h2>
            <p>{} products — rendered server-side in pure Rust — zero JavaScript frameworks</p></div>"#,
            products.len()
        ));
    }

    // Search bar
    body.push_str(&format!(
        r#"<form class="search-bar" method="GET" action="/">
        <input type="text" name="q" placeholder="Search products..." value="{q}">
        <select name="category"><option value="">All Categories</option>{cat_opts}</select>
        <select name="sort">
            <option value="">Sort by</option>
            <option value="price_asc">Price: Low → High</option>
            <option value="price_desc">Price: High → Low</option>
            <option value="rating">Top Rated</option>
            <option value="name">Name A-Z</option>
        </select>
        <input type="number" name="min_price" placeholder="Min $" value="{min_p}" style="width:100px">
        <input type="number" name="max_price" placeholder="Max $" value="{max_p}" style="width:100px">
        <button class="btn" type="submit">Search</button></form>"#,
        q = query.q.as_deref().unwrap_or(""),
        min_p = query.min_price.map(|p| p.to_string()).unwrap_or_default(),
        max_p = query.max_price.map(|p| p.to_string()).unwrap_or_default(),
        cat_opts = categories.iter().map(|c| {
            let sel = if query.category.as_deref() == Some(c) { " selected" } else { "" };
            format!(r#"<option value="{c}"{sel}>{c}</option>"#)
        }).collect::<String>(),
    ));

    // Category pills
    body.push_str(r#"<div class="cat-filter">"#);
    body.push_str(r#"<a href="/">All</a>"#);
    for cat in &categories {
        let active = if query.category.as_deref() == Some(cat) { " active" } else { "" };
        body.push_str(&format!(r#"<a href="/?category={cat}" class="{active}">{cat}</a>"#));
    }
    body.push_str("</div>");

    // Featured section
    if !featured.is_empty() && query.q.is_none() && query.category.is_none() {
        body.push_str("<h2 style=\"margin:1rem 0\">Featured</h2>");
        body.push_str(r#"<div class="grid">"#);
        for p in &featured {
            body.push_str(&product_card(p));
        }
        body.push_str("</div>");
        body.push_str("<h2 style=\"margin:1.5rem 0 1rem\">All Products</h2>");
    } else {
        body.push_str(&format!("<h2 style=\"margin:1rem 0\">{} results</h2>", filtered.len()));
    }

    // Product grid
    body.push_str(r#"<div class="grid">"#);
    for p in &filtered {
        body.push_str(&product_card(p));
    }
    body.push_str("</div>");

    page("Shop", &nav(cart_count), &body, start.elapsed().as_micros())
}

fn product_card(p: &Product) -> String {
    let stock_class = if p.stock < 10 { "stock-low" } else { "stock-ok" };
    let stock_text = if p.stock < 10 { format!("Only {} left!", p.stock) } else { format!("{} in stock", p.stock) };
    let stars = "★".repeat(p.rating as usize) + &"☆".repeat(5 - p.rating as usize);
    let featured_badge = if p.featured { r#"<span class="featured-badge">FEATURED</span>"# } else { "" };
    format!(
        r#"<div class="card">
        <div class="emoji">{emoji}</div>
        <h3>{name}{featured_badge}</h3>
        <div class="price">${price:.2}</div>
        <div class="desc">{desc}</div>
        <div class="meta"><span class="stars">{stars}</span> <span>({reviews})</span></div>
        <div class="meta"><span class="{stock_class}">{stock_text}</span></div>
        <div style="margin-top:0.75rem;display:flex;gap:0.5rem">
            <a href="/product/{id}" class="btn btn-outline" style="flex:1;text-align:center">Details</a>
            <form method="POST" action="/cart/add/{id}" style="flex:1"><button class="btn" style="width:100%">Add to Cart</button></form>
        </div></div>"#,
        emoji = p.image_emoji, name = p.name, price = p.price, desc = p.description,
        reviews = p.reviews, id = p.id,
    )
}

async fn product_detail(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let products = state.products.read().await;
    let carts = state.carts.read().await;
    let cart_count: usize = carts.get("default").map(|c| c.iter().map(|i| i.quantity as usize).sum()).unwrap_or(0);

    match products.iter().find(|p| p.id == id) {
        Some(p) => {
            let stars = "★".repeat(p.rating as usize) + &"☆".repeat(5 - p.rating as usize);
            let related: String = products.iter()
                .filter(|r| r.category == p.category && r.id != p.id)
                .take(3)
                .map(|r| format!(r#"<a href="/product/{}" class="card" style="text-decoration:none;display:block"><div class="emoji" style="font-size:2rem">{}</div><h3 style="font-size:0.9rem">{}</h3><div class="price">${:.2}</div></a>"#, r.id, r.image_emoji, r.name, r.price))
                .collect();

            let body = format!(
                r#"<div style="display:grid;grid-template-columns:1fr 1fr;gap:2rem;margin-top:1rem">
                <div class="card" style="text-align:center;padding:3rem"><div style="font-size:8rem">{emoji}</div></div>
                <div>
                    <h1 style="color:#f0f6fc;margin-bottom:0.5rem">{name}</h1>
                    <div class="price" style="font-size:2rem;margin:1rem 0">${price:.2}</div>
                    <div class="stars" style="font-size:1.2rem">{stars} <span style="color:#8b949e">({reviews} reviews)</span></div>
                    <p style="margin:1.5rem 0;line-height:1.6">{desc}</p>
                    <div style="margin:1rem 0;color:{stock_color}">{stock_text}</div>
                    <div>Category: <a href="/?category={cat}">{cat}</a></div>
                    <form method="POST" action="/cart/add/{id}" style="margin-top:1.5rem">
                        <button class="btn" style="font-size:1.1rem;padding:0.75rem 2rem">Add to Cart</button>
                    </form>
                </div></div>
                <h2 style="margin:2rem 0 1rem">Related in {cat}</h2>
                <div class="grid">{related}</div>"#,
                emoji = p.image_emoji, name = p.name, price = p.price, stars = stars,
                reviews = p.reviews, desc = p.description, cat = p.category, id = p.id,
                stock_color = if p.stock < 10 { "#da3633" } else { "#3fb950" },
                stock_text = if p.stock < 10 { format!("Only {} left — order soon!", p.stock) } else { format!("{} in stock", p.stock) },
            );
            page(&p.name, &nav(cart_count), &body, start.elapsed().as_micros()).into_response()
        }
        None => (StatusCode::NOT_FOUND, Html("<h1>Product not found</h1>".to_string())).into_response(),
    }
}

async fn add_to_cart(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Redirect {
    let mut carts = state.carts.write().await;
    let cart = carts.entry("default".to_string()).or_default();
    if let Some(item) = cart.iter_mut().find(|i| i.product_id == id) {
        item.quantity += 1;
    } else {
        cart.push(CartItem { product_id: id, quantity: 1 });
    }
    Redirect::to("/cart")
}

async fn view_cart(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let products = state.products.read().await;
    let carts = state.carts.read().await;
    let cart = carts.get("default");
    let items: Vec<CartItem> = cart.cloned().unwrap_or_default();
    let cart_count: usize = items.iter().map(|i| i.quantity as usize).sum();

    let mut body = String::from("<h1 style=\"color:#f0f6fc;margin-bottom:1.5rem\">Shopping Cart</h1>");

    if items.is_empty() {
        body.push_str(r#"<div class="card" style="text-align:center;padding:3rem"><p style="font-size:1.2rem">Your cart is empty</p><a href="/" class="btn" style="margin-top:1rem;display:inline-block">Start Shopping</a></div>"#);
    } else {
        body.push_str("<table><thead><tr><th>Product</th><th>Price</th><th>Qty</th><th>Subtotal</th><th></th></tr></thead><tbody>");
        let mut total = 0.0;
        for item in &items {
            if let Some(p) = products.iter().find(|p| p.id == item.product_id) {
                let subtotal = p.price * item.quantity as f64;
                total += subtotal;
                body.push_str(&format!(
                    r#"<tr><td>{} {}</td><td>${:.2}</td><td>{}</td><td style="color:#3fb950">${:.2}</td>
                    <td><form method="POST" action="/cart/remove/{}" style="display:inline"><button class="btn btn-danger" style="padding:0.25rem 0.5rem;font-size:0.8rem">Remove</button></form></td></tr>"#,
                    p.image_emoji, p.name, p.price, item.quantity, subtotal, p.id
                ));
            }
        }
        body.push_str(&format!(
            r#"</tbody><tfoot><tr class="total-row"><td colspan="3">Total</td><td>${total:.2}</td><td></td></tr></tfoot></table>
            <div style="margin-top:1.5rem;display:flex;gap:1rem;justify-content:flex-end">
                <a href="/" class="btn btn-outline">Continue Shopping</a>
                <form method="POST" action="/checkout"><button class="btn" style="font-size:1.1rem;padding:0.75rem 2rem">Checkout — ${total:.2}</button></form>
            </div>"#
        ));
    }

    page("Cart", &nav(cart_count), &body, start.elapsed().as_micros())
}

async fn remove_from_cart(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Redirect {
    let mut carts = state.carts.write().await;
    if let Some(cart) = carts.get_mut("default") {
        cart.retain(|i| i.product_id != id);
    }
    Redirect::to("/cart")
}

async fn checkout(State(state): State<Arc<AppState>>) -> Redirect {
    let products = state.products.read().await;
    let mut carts = state.carts.write().await;
    let mut orders = state.orders.write().await;
    let mut next_id = state.next_order_id.write().await;

    if let Some(cart) = carts.remove("default") {
        if !cart.is_empty() {
            let mut items = Vec::new();
            let mut total = 0.0;
            for ci in &cart {
                if let Some(p) = products.iter().find(|p| p.id == ci.product_id) {
                    let sub = p.price * ci.quantity as f64;
                    total += sub;
                    items.push((p.name.clone(), ci.quantity, p.price));
                }
            }
            orders.push(Order {
                id: *next_id,
                items,
                total,
                status: "Confirmed".into(),
                created_at: chrono_now(),
            });
            *next_id += 1;
        }
    }
    Redirect::to("/orders")
}

async fn view_orders(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let orders = state.orders.read().await;
    let carts = state.carts.read().await;
    let cart_count: usize = carts.get("default").map(|c| c.iter().map(|i| i.quantity as usize).sum()).unwrap_or(0);

    let mut body = String::from("<h1 style=\"color:#f0f6fc;margin-bottom:1.5rem\">Order History</h1>");

    if orders.is_empty() {
        body.push_str(r#"<div class="card" style="text-align:center;padding:3rem"><p>No orders yet</p><a href="/" class="btn" style="margin-top:1rem;display:inline-block">Start Shopping</a></div>"#);
    } else {
        for order in orders.iter().rev() {
            body.push_str(&format!(
                r#"<div class="card" style="margin-bottom:1rem">
                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:1rem">
                    <h3 style="color:#f0f6fc">Order #{id}</h3>
                    <span class="order-status status-confirmed">{status}</span>
                </div>
                <div style="color:#6e7681;font-size:0.85rem;margin-bottom:0.75rem">{created}</div>
                <table><thead><tr><th>Item</th><th>Qty</th><th>Price</th></tr></thead><tbody>"#,
                id = order.id, status = order.status, created = order.created_at
            ));
            for (name, qty, price) in &order.items {
                body.push_str(&format!("<tr><td>{name}</td><td>{qty}</td><td>${price:.2}</td></tr>"));
            }
            body.push_str(&format!(
                r#"</tbody></table><div style="text-align:right;margin-top:0.75rem" class="total-row">Total: ${:.2}</div></div>"#,
                order.total
            ));
        }
    }

    page("Orders", &nav(cart_count), &body, start.elapsed().as_micros())
}

async fn api_products(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let products = state.products.read().await;
    let data: Vec<Value> = products.iter().map(|p| json!({
        "id": p.id, "name": p.name, "price": p.price, "category": p.category,
        "stock": p.stock, "rating": p.rating, "reviews": p.reviews, "featured": p.featured,
    })).collect();
    let elapsed = start.elapsed();
    axum::Json(json!({ "products": data, "count": data.len(), "latency_us": elapsed.as_micros() }))
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("2026-02-08T{h:02}:{m:02}:{s:02}Z")
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        products: RwLock::new(seed_products()),
        carts: RwLock::new(HashMap::new()),
        orders: RwLock::new(Vec::new()),
        next_order_id: RwLock::new(1001),
    });

    let products = state.products.read().await;
    eprintln!("ecommerce ready: {} products, {} categories",
        products.len(),
        { let mut c: Vec<_> = products.iter().map(|p| &p.category).collect(); c.sort(); c.dedup(); c.len() }
    );
    drop(products);

    let app = Router::new()
        .route("/", get(storefront))
        .route("/product/{id}", get(product_detail))
        .route("/cart", get(view_cart))
        .route("/cart/add/{id}", post(add_to_cart))
        .route("/cart/remove/{id}", post(remove_from_cart))
        .route("/checkout", post(checkout))
        .route("/orders", get(view_orders))
        .route("/api/products", get(api_products))
        .with_state(state);

    let addr = "0.0.0.0:4001";
    eprintln!("ecommerce listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
