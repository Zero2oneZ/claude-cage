//! Real Estate Seller's Business Site — listings, agents, search, inquiries, mortgage calc.
//! Pure Rust, server-rendered HTML, in-memory state, zero JS frameworks.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct Property {
    id: u32,
    title: String,
    address: String,
    city: String,
    state: String,
    zip: String,
    price: u64,
    beds: u8,
    baths: f32,
    sqft: u32,
    lot_sqft: u32,
    year_built: u16,
    property_type: String,
    status: String,
    description: String,
    features: Vec<String>,
    images_emoji: String,
    agent_id: u32,
    days_on_market: u32,
    price_per_sqft: f64,
    hoa: Option<u32>,
}

#[derive(Clone)]
struct Agent {
    id: u32,
    name: String,
    title: String,
    avatar: String,
    phone: String,
    email: String,
    bio: String,
    listings: u32,
    sold: u32,
    rating: f32,
    reviews: u32,
    specialties: Vec<String>,
    years_experience: u8,
}

#[derive(Clone)]
struct Inquiry {
    id: u32,
    property_id: u32,
    name: String,
    email: String,
    phone: String,
    message: String,
    created_at: String,
}

struct AppState {
    properties: RwLock<Vec<Property>>,
    agents: RwLock<Vec<Agent>>,
    inquiries: RwLock<Vec<Inquiry>>,
    next_inquiry_id: RwLock<u32>,
}

fn seed_properties() -> Vec<Property> {
    vec![
        Property { id: 1, title: "Modern Craftsman Home".into(), address: "1247 Oak Valley Dr".into(), city: "Austin".into(), state: "TX".into(), zip: "78704".into(), price: 685000, beds: 4, baths: 3.0, sqft: 2450, lot_sqft: 8500, year_built: 2019, property_type: "Single Family".into(), status: "Active".into(), description: "Stunning modern craftsman with open floor plan, chef's kitchen with quartz counters, primary suite with spa bath. Covered patio overlooks landscaped backyard with mature oaks.".into(), features: vec!["Open Floor Plan".into(), "Smart Home".into(), "Solar Panels".into(), "EV Charger".into()], images_emoji: "🏡".into(), agent_id: 1, days_on_market: 5, price_per_sqft: 279.59, hoa: None },
        Property { id: 2, title: "Downtown Luxury Condo".into(), address: "500 West 2nd St #1804".into(), city: "Austin".into(), state: "TX".into(), zip: "78701".into(), price: 925000, beds: 2, baths: 2.5, sqft: 1680, lot_sqft: 0, year_built: 2022, property_type: "Condo".into(), status: "Active".into(), description: "18th-floor corner unit with floor-to-ceiling windows, panoramic skyline views. Italian marble baths, Miele appliances, concierge, rooftop pool, fitness center.".into(), features: vec!["City Views".into(), "Concierge".into(), "Rooftop Pool".into(), "Gym".into()], images_emoji: "🏙️".into(), agent_id: 2, days_on_market: 12, price_per_sqft: 550.60, hoa: Some(850) },
        Property { id: 3, title: "Hill Country Estate".into(), address: "8900 Ranch Road 620".into(), city: "Bee Cave".into(), state: "TX".into(), zip: "78738".into(), price: 2150000, beds: 6, baths: 5.5, sqft: 5800, lot_sqft: 43560, year_built: 2017, property_type: "Single Family".into(), status: "Active".into(), description: "Magnificent estate on 1 acre with infinity pool, outdoor kitchen, wine cellar, home theater, and panoramic hill country views. Gated community with private trails.".into(), features: vec!["Infinity Pool".into(), "Wine Cellar".into(), "Theater".into(), "Gated".into()], images_emoji: "🏰".into(), agent_id: 1, days_on_market: 21, price_per_sqft: 370.69, hoa: Some(350) },
        Property { id: 4, title: "Charming Bungalow".into(), address: "2105 E Cesar Chavez St".into(), city: "Austin".into(), state: "TX".into(), zip: "78702".into(), price: 495000, beds: 2, baths: 1.0, sqft: 1100, lot_sqft: 5200, year_built: 1945, property_type: "Single Family".into(), status: "Active".into(), description: "Original 1945 bungalow with character. Hardwood floors, updated kitchen, detached studio/ADU potential. Walk to East Austin dining and entertainment.".into(), features: vec!["Hardwood Floors".into(), "ADU Potential".into(), "Walkable".into(), "Character".into()], images_emoji: "🏠".into(), agent_id: 3, days_on_market: 3, price_per_sqft: 450.00, hoa: None },
        Property { id: 5, title: "Lakefront Retreat".into(), address: "1500 Lakeshore Dr".into(), city: "Lakeway".into(), state: "TX".into(), zip: "78734".into(), price: 1750000, beds: 5, baths: 4.0, sqft: 4200, lot_sqft: 21780, year_built: 2015, property_type: "Single Family".into(), status: "Pending".into(), description: "Directly on Lake Travis with private dock and boat lift. Open living with walls of glass, chef's kitchen, resort-style pool, outdoor living pavilion.".into(), features: vec!["Lake Access".into(), "Private Dock".into(), "Pool".into(), "Boat Lift".into()], images_emoji: "🏖️".into(), agent_id: 2, days_on_market: 8, price_per_sqft: 416.67, hoa: Some(200) },
        Property { id: 6, title: "New Construction Townhome".into(), address: "3300 S Lamar Blvd #102".into(), city: "Austin".into(), state: "TX".into(), zip: "78704".into(), price: 545000, beds: 3, baths: 2.5, sqft: 1850, lot_sqft: 0, year_built: 2026, property_type: "Townhome".into(), status: "Active".into(), description: "Brand new construction! Energy-efficient townhome with rooftop deck, 2-car garage, walk-in closets, designer finishes throughout. Minutes to Barton Springs.".into(), features: vec!["New Build".into(), "Rooftop Deck".into(), "2-Car Garage".into(), "Energy Star".into()], images_emoji: "🏗️".into(), agent_id: 3, days_on_market: 1, price_per_sqft: 294.59, hoa: Some(175) },
        Property { id: 7, title: "Mid-Century Modern Gem".into(), address: "4510 Balcones Dr".into(), city: "Austin".into(), state: "TX".into(), zip: "78731".into(), price: 875000, beds: 3, baths: 2.0, sqft: 2100, lot_sqft: 9800, year_built: 1962, property_type: "Single Family".into(), status: "Active".into(), description: "Beautifully restored mid-century modern. Original terrazzo floors, walls of glass, flat roof, breezeway. Updated mechanicals, new roof. Mature landscaping.".into(), features: vec!["Mid-Century".into(), "Terrazzo".into(), "Renovated".into(), "Private Lot".into()], images_emoji: "🏛️".into(), agent_id: 1, days_on_market: 14, price_per_sqft: 416.67, hoa: None },
        Property { id: 8, title: "Investment Duplex".into(), address: "900 E 51st St".into(), city: "Austin".into(), state: "TX".into(), zip: "78751".into(), price: 620000, beds: 4, baths: 2.0, sqft: 1800, lot_sqft: 6000, year_built: 1978, property_type: "Multi-Family".into(), status: "Active".into(), description: "Income-producing duplex near UT campus. Both units updated, separately metered. Current rental income $4,200/mo. Strong rental history, low vacancy.".into(), features: vec!["Rental Income".into(), "Near UT".into(), "Updated".into(), "Dual Units".into()], images_emoji: "🏢".into(), agent_id: 2, days_on_market: 7, price_per_sqft: 344.44, hoa: None },
        Property { id: 9, title: "Luxury Penthouse".into(), address: "200 Congress Ave #PH2".into(), city: "Austin".into(), state: "TX".into(), zip: "78701".into(), price: 3200000, beds: 3, baths: 3.5, sqft: 3400, lot_sqft: 0, year_built: 2021, property_type: "Condo".into(), status: "Active".into(), description: "Full-floor penthouse with private elevator, 360-degree views, Sub-Zero/Wolf kitchen, marble throughout, smart glass, 3 terraces. White-glove building services.".into(), features: vec!["Full Floor".into(), "Private Elevator".into(), "360 Views".into(), "Terraces".into()], images_emoji: "✨".into(), agent_id: 1, days_on_market: 30, price_per_sqft: 941.18, hoa: Some(2200) },
        Property { id: 10, title: "Family Ranch Home".into(), address: "15200 FM 1826".into(), city: "Driftwood".into(), state: "TX".into(), zip: "78619".into(), price: 1100000, beds: 4, baths: 3.0, sqft: 3200, lot_sqft: 217800, year_built: 2008, property_type: "Single Family".into(), status: "Active".into(), description: "5-acre ranch with custom home, barn, workshop, and fenced pastures. Wrap-around porch, stone fireplace, well water. Perfect for horses or hobby farming.".into(), features: vec!["5 Acres".into(), "Barn".into(), "Workshop".into(), "Horse-Ready".into()], images_emoji: "🐴".into(), agent_id: 3, days_on_market: 18, price_per_sqft: 343.75, hoa: None },
    ]
}

fn seed_agents() -> Vec<Agent> {
    vec![
        Agent { id: 1, name: "Sarah Chen".into(), title: "Principal Broker".into(), avatar: "👩‍💼".into(), phone: "(512) 555-0101".into(), email: "sarah@rustestates.com".into(), bio: "Top 1% producer in Austin for 8 consecutive years. Specializing in luxury residential and investment properties. UT McCombs MBA.".into(), listings: 24, sold: 312, rating: 4.9, reviews: 189, specialties: vec!["Luxury".into(), "Investment".into(), "New Construction".into()], years_experience: 15 },
        Agent { id: 2, name: "Marcus Williams".into(), title: "Senior Agent".into(), avatar: "👨‍💼".into(), phone: "(512) 555-0102".into(), email: "marcus@rustestates.com".into(), bio: "Former tech executive turned real estate professional. Deep expertise in downtown condos and lakefront properties. Data-driven approach to pricing.".into(), listings: 18, sold: 156, rating: 4.8, reviews: 112, specialties: vec!["Condos".into(), "Lakefront".into(), "Tech Corridor".into()], years_experience: 8 },
        Agent { id: 3, name: "Elena Rodriguez".into(), title: "Buyer's Specialist".into(), avatar: "👩‍💻".into(), phone: "(512) 555-0103".into(), email: "elena@rustestates.com".into(), bio: "Passionate about helping first-time buyers and families find their perfect home. Fluent in English and Spanish. Community-focused approach.".into(), listings: 12, sold: 98, rating: 4.7, reviews: 87, specialties: vec!["First-Time Buyers".into(), "Families".into(), "East Austin".into()], years_experience: 6 },
    ]
}

const CSS: &str = r#"
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0d1117;color:#c9d1d9;font-family:'Segoe UI',system-ui,sans-serif}
a{color:#58a6ff;text-decoration:none}a:hover{text-decoration:underline}
.nav{background:#161b22;border-bottom:1px solid #30363d;padding:0.75rem 2rem;display:flex;align-items:center;gap:2rem}
.nav h1{font-size:1.2rem;color:#f0f6fc}.nav a{color:#8b949e}.nav a:hover{color:#f0f6fc}
.container{max-width:1200px;margin:0 auto;padding:1.5rem}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(350px,1fr));gap:1.25rem}
.card{background:#161b22;border:1px solid #30363d;border-radius:8px;overflow:hidden;transition:border-color .2s}
.card:hover{border-color:#58a6ff}
.listing-img{background:linear-gradient(135deg,#1a1a2e 0%,#16213e 100%);padding:3rem;text-align:center;position:relative}
.listing-img .emoji{font-size:4rem}
.status-badge{position:absolute;top:12px;left:12px;padding:4px 12px;border-radius:4px;font-size:0.8rem;font-weight:600}
.status-active{background:#238636;color:#fff}.status-pending{background:#d29922;color:#fff}.status-sold{background:#da3633;color:#fff}
.dom-badge{position:absolute;top:12px;right:12px;background:rgba(0,0,0,0.7);color:#c9d1d9;padding:4px 10px;border-radius:4px;font-size:0.75rem}
.card-body{padding:1.25rem}
.card-body .price{color:#3fb950;font-size:1.5rem;font-weight:700}
.card-body h3{color:#f0f6fc;margin:0.5rem 0 0.25rem;font-size:1.1rem}
.card-body .address{color:#8b949e;font-size:0.9rem}
.card-body .specs{display:flex;gap:1rem;margin:0.75rem 0;color:#c9d1d9;font-size:0.9rem}
.card-body .specs span{display:flex;align-items:center;gap:4px}
.features{display:flex;gap:0.4rem;flex-wrap:wrap;margin-top:0.5rem}
.feature{background:#21262d;color:#8b949e;padding:2px 8px;border-radius:4px;font-size:0.75rem}
.btn{background:#238636;color:#fff;border:none;padding:0.5rem 1rem;border-radius:6px;cursor:pointer;font-size:0.9rem}
.btn:hover{background:#2ea043}.btn-outline{background:transparent;border:1px solid #30363d;color:#c9d1d9}.btn-outline:hover{border-color:#58a6ff}
input,select,textarea{background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:6px;padding:0.5rem;font-size:0.9rem;width:100%}
.search-bar{display:flex;gap:0.75rem;margin-bottom:1.5rem;flex-wrap:wrap}
.search-bar input,.search-bar select{min-width:120px}
.hero{background:linear-gradient(135deg,#1a1a2e 0%,#0d1117 100%);border:1px solid #30363d;border-radius:12px;padding:3rem;margin-bottom:2rem;text-align:center}
.hero h2{color:#f0f6fc;font-size:2rem;margin-bottom:0.5rem}.hero p{color:#8b949e;font-size:1.1rem}
.stats-row{display:grid;grid-template-columns:repeat(4,1fr);gap:1rem;margin:2rem 0}
.stat-card{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:1.25rem;text-align:center}
.stat-card .number{font-size:1.8rem;font-weight:700;color:#f0f6fc}.stat-card .label{color:#8b949e;font-size:0.85rem}
.agent-card{display:flex;gap:1.5rem;padding:1.25rem;background:#161b22;border:1px solid #30363d;border-radius:8px;margin-bottom:1rem}
.agent-card .avatar{font-size:3.5rem}
.agent-card h3{color:#f0f6fc}.agent-card .title{color:#8b949e;font-size:0.9rem}
.stars{color:#f0c000}
.mortgage{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:1.5rem;margin-top:1.5rem}
table{width:100%;border-collapse:collapse}th,td{padding:0.75rem;text-align:left;border-bottom:1px solid #21262d}
th{color:#8b949e;font-weight:600;font-size:0.85rem}
.speed{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:0.5rem 1rem;margin-top:1rem;text-align:center;color:#6e7681;font-size:0.8rem}
.speed strong{color:#3fb950}
.form-group{margin-bottom:1rem}
.form-group label{display:block;color:#8b949e;font-size:0.85rem;margin-bottom:0.25rem}
.success-msg{background:#1a4731;border:1px solid #238636;color:#3fb950;padding:1rem;border-radius:8px;margin-bottom:1rem}
"#;

fn nav_html() -> &'static str {
    r#"<nav class="nav"><h1>🏠 RustEstates</h1>
    <a href="/">Listings</a><a href="/agents">Agents</a><a href="/calculator">Mortgage Calc</a>
    <a href="/inquiries">Inquiries</a><a href="/api/listings">API</a></nav>"#
}

fn speed_bar(us: u128) -> String {
    format!(r#"<div class="speed">Rendered in <strong>{us}μs</strong> — Pure Rust, zero JS frameworks</div>"#)
}

fn wrap(title: &str, body: &str, us: u128) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>{title} — RustEstates</title><style>{CSS}</style></head>
        <body>{nav}{body}{speed}</body></html>"#,
        nav = nav_html(), speed = speed_bar(us)
    ))
}

fn format_price(p: u64) -> String {
    if p >= 1_000_000 {
        format!("${:.2}M", p as f64 / 1_000_000.0)
    } else {
        let s = p.to_string();
        let mut r = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 { r.push(','); }
            r.push(c);
        }
        format!("${}", r.chars().rev().collect::<String>())
    }
}

fn listing_card(p: &Property) -> String {
    let status_class = match p.status.as_str() {
        "Active" => "status-active", "Pending" => "status-pending", _ => "status-sold"
    };
    format!(
        r#"<a href="/listing/{id}" class="card" style="text-decoration:none;display:block">
        <div class="listing-img"><div class="emoji">{emoji}</div>
            <span class="status-badge {status_class}">{status}</span>
            <span class="dom-badge">{dom}d on market</span>
        </div>
        <div class="card-body">
            <div class="price">{price}</div>
            <h3>{title}</h3>
            <div class="address">{address}, {city}, {state} {zip}</div>
            <div class="specs">
                <span>🛏️ {beds} bd</span><span>🚿 {baths} ba</span>
                <span>📐 {sqft} sqft</span><span>💲{ppsf:.0}/sqft</span>
            </div>
            <div class="features">{features}</div>
        </div></a>"#,
        id = p.id, emoji = p.images_emoji, status = p.status, dom = p.days_on_market,
        price = format_price(p.price), title = p.title,
        address = p.address, city = p.city, state = p.state, zip = p.zip,
        beds = p.beds, baths = p.baths, sqft = p.sqft, ppsf = p.price_per_sqft,
        features = p.features.iter().take(3).map(|f| format!(r#"<span class="feature">{f}</span>"#)).collect::<String>(),
    )
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    property_type: Option<String>,
    min_price: Option<u64>,
    max_price: Option<u64>,
    beds: Option<u8>,
    sort: Option<String>,
}

async fn listings(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Html<String> {
    let start = std::time::Instant::now();
    let properties = state.properties.read().await;

    let mut filtered: Vec<&Property> = properties.iter().filter(|p| {
        let q_match = query.q.as_ref().map(|q| {
            let q = q.to_lowercase();
            p.title.to_lowercase().contains(&q) || p.address.to_lowercase().contains(&q)
                || p.city.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q)
        }).unwrap_or(true);
        let type_match = query.property_type.as_ref().map(|t| t == &p.property_type).unwrap_or(true);
        let min_match = query.min_price.map(|m| p.price >= m).unwrap_or(true);
        let max_match = query.max_price.map(|m| p.price <= m).unwrap_or(true);
        let bed_match = query.beds.map(|b| p.beds >= b).unwrap_or(true);
        q_match && type_match && min_match && max_match && bed_match
    }).collect();

    match query.sort.as_deref() {
        Some("price_asc") => filtered.sort_by_key(|p| p.price),
        Some("price_desc") => filtered.sort_by(|a, b| b.price.cmp(&a.price)),
        Some("newest") => filtered.sort_by_key(|p| p.days_on_market),
        Some("sqft") => filtered.sort_by(|a, b| b.sqft.cmp(&a.sqft)),
        _ => filtered.sort_by_key(|p| p.days_on_market),
    }

    let active = properties.iter().filter(|p| p.status == "Active").count();
    let avg_price = if !properties.is_empty() { properties.iter().map(|p| p.price).sum::<u64>() / properties.len() as u64 } else { 0 };
    let total_sqft: u32 = properties.iter().map(|p| p.sqft).sum();

    let types: Vec<String> = { let mut t: Vec<_> = properties.iter().map(|p| p.property_type.clone()).collect(); t.sort(); t.dedup(); t };

    let mut body = format!(
        r#"<div class="container">
        <div class="hero"><h2>Find Your Dream Home</h2><p>{total} listings in the Austin metro area</p></div>
        <div class="stats-row">
            <div class="stat-card"><div class="number">{active}</div><div class="label">Active Listings</div></div>
            <div class="stat-card"><div class="number">{avg}</div><div class="label">Avg. Price</div></div>
            <div class="stat-card"><div class="number">{sqft}</div><div class="label">Total Sq Ft</div></div>
            <div class="stat-card"><div class="number">{filtered_count}</div><div class="label">Matching</div></div>
        </div>
        <form class="search-bar" method="GET" action="/">
            <input type="text" name="q" placeholder="Search address, city, keywords..." value="{q}" style="flex:2">
            <select name="property_type"><option value="">All Types</option>{type_opts}</select>
            <input type="number" name="min_price" placeholder="Min $" value="{min_p}" style="max-width:120px">
            <input type="number" name="max_price" placeholder="Max $" value="{max_p}" style="max-width:120px">
            <select name="beds"><option value="">Beds</option><option value="1">1+</option><option value="2">2+</option><option value="3">3+</option><option value="4">4+</option><option value="5">5+</option></select>
            <select name="sort"><option value="">Newest</option><option value="price_asc">Price ↑</option><option value="price_desc">Price ↓</option><option value="sqft">Largest</option></select>
            <button class="btn" type="submit">Search</button>
        </form>
        <div class="grid">"#,
        total = properties.len(), avg = format_price(avg_price),
        sqft = format!("{:.0}K", total_sqft as f64 / 1000.0),
        filtered_count = filtered.len(),
        q = query.q.as_deref().unwrap_or(""),
        min_p = query.min_price.map(|p| p.to_string()).unwrap_or_default(),
        max_p = query.max_price.map(|p| p.to_string()).unwrap_or_default(),
        type_opts = types.iter().map(|t| format!(r#"<option value="{t}">{t}</option>"#)).collect::<String>(),
    );

    for p in &filtered { body.push_str(&listing_card(p)); }
    body.push_str("</div></div>");

    wrap("Listings", &body, start.elapsed().as_micros())
}

async fn listing_detail(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let properties = state.properties.read().await;
    let agents = state.agents.read().await;

    match properties.iter().find(|p| p.id == id) {
        Some(p) => {
            let agent = agents.iter().find(|a| a.id == p.agent_id);
            let similar: String = properties.iter()
                .filter(|s| s.property_type == p.property_type && s.id != p.id)
                .take(3)
                .map(|s| listing_card(s))
                .collect();

            let agent_html = agent.map(|a| format!(
                r#"<div class="agent-card"><div class="avatar">{}</div><div>
                <h3>{}</h3><div class="title">{}</div>
                <div style="margin:0.5rem 0">{} · {}</div>
                <div class="stars">{} <span style="color:#8b949e">({} reviews)</span></div>
                </div></div>"#,
                a.avatar, a.name, a.title, a.phone, a.email,
                "★".repeat(a.rating as usize) + &"☆".repeat(5 - a.rating as usize), a.reviews
            )).unwrap_or_default();

            // Mortgage estimate
            let down = p.price as f64 * 0.20;
            let loan = p.price as f64 - down;
            let rate = 0.065 / 12.0;
            let n = 360.0;
            let monthly = loan * (rate * (1.0_f64 + rate).powf(n)) / ((1.0_f64 + rate).powf(n) - 1.0);
            let hoa_monthly = p.hoa.unwrap_or(0) as f64;

            let body = format!(
                r#"<div class="container"><div style="display:grid;grid-template-columns:2fr 1fr;gap:2rem">
                <div>
                    <div class="card" style="margin-bottom:1.5rem">
                        <div class="listing-img" style="padding:4rem"><div class="emoji" style="font-size:6rem">{emoji}</div>
                            <span class="status-badge {status_class}">{status}</span></div>
                        <div class="card-body">
                            <div class="price" style="font-size:2.5rem">{price}</div>
                            <h1 style="color:#f0f6fc;margin:0.5rem 0">{title}</h1>
                            <div class="address" style="font-size:1.1rem">{address}, {city}, {state} {zip}</div>
                            <div class="specs" style="font-size:1.1rem;margin:1rem 0">
                                <span>🛏️ {beds} beds</span><span>🚿 {baths} baths</span>
                                <span>📐 {sqft} sqft</span><span>📅 Built {year}</span>
                                <span>🏷️ {ptype}</span>
                            </div>
                            <p style="margin:1.5rem 0;line-height:1.7;font-size:1.05rem">{desc}</p>
                            <div class="features" style="margin:1rem 0">{features}</div>
                            <div style="margin-top:1rem;color:#8b949e">
                                <span>💲{ppsf:.0}/sqft</span> · <span>🏡 Lot: {lot}</span>
                                {hoa_text} · <span>📅 {dom} days on market</span>
                            </div>
                        </div>
                    </div>
                    <h2 style="color:#f0f6fc;margin:1.5rem 0 1rem">Similar Properties</h2>
                    <div class="grid">{similar}</div>
                </div>
                <div>
                    <h3 style="color:#f0f6fc;margin-bottom:1rem">Listing Agent</h3>
                    {agent_html}
                    <div class="mortgage">
                        <h3 style="color:#f0f6fc;margin-bottom:1rem">💰 Mortgage Estimate</h3>
                        <table>
                            <tr><td>Home Price</td><td style="text-align:right">{price}</td></tr>
                            <tr><td>Down Payment (20%)</td><td style="text-align:right">{down}</td></tr>
                            <tr><td>Loan Amount</td><td style="text-align:right">{loan}</td></tr>
                            <tr><td>Interest Rate</td><td style="text-align:right">6.5%</td></tr>
                            <tr><td>Monthly P&I</td><td style="text-align:right;color:#3fb950;font-weight:700">${monthly:.0}/mo</td></tr>
                            <tr><td>Est. Total (w/ HOA)</td><td style="text-align:right;color:#3fb950;font-weight:700">${total_monthly:.0}/mo</td></tr>
                        </table>
                    </div>
                    <div class="card" style="margin-top:1rem;padding:1.5rem">
                        <h3 style="color:#f0f6fc;margin-bottom:1rem">📧 Schedule a Showing</h3>
                        <form method="POST" action="/inquiry/{id}">
                            <div class="form-group"><label>Name</label><input type="text" name="name" required></div>
                            <div class="form-group"><label>Email</label><input type="email" name="email" required></div>
                            <div class="form-group"><label>Phone</label><input type="tel" name="phone"></div>
                            <div class="form-group"><label>Message</label><textarea name="message" rows="3" placeholder="I'd like to schedule a showing..."></textarea></div>
                            <button class="btn" style="width:100%">Send Inquiry</button>
                        </form>
                    </div>
                </div></div></div>"#,
                emoji = p.images_emoji, title = p.title, price = format_price(p.price),
                address = p.address, city = p.city, state = p.state, zip = p.zip,
                beds = p.beds, baths = p.baths, sqft = p.sqft, year = p.year_built,
                ptype = p.property_type, desc = p.description, dom = p.days_on_market,
                ppsf = p.price_per_sqft, lot = if p.lot_sqft > 0 { format!("{} sqft", p.lot_sqft) } else { "N/A".into() },
                hoa_text = p.hoa.map(|h| format!(" · <span>HOA: ${h}/mo</span>")).unwrap_or_default(),
                features = p.features.iter().map(|f| format!(r#"<span class="feature">{f}</span>"#)).collect::<String>(),
                status = p.status,
                status_class = match p.status.as_str() { "Active" => "status-active", "Pending" => "status-pending", _ => "status-sold" },
                down = format_price((down) as u64), loan = format_price(loan as u64),
                monthly = monthly, total_monthly = monthly + hoa_monthly,
                id = p.id,
            );
            wrap(&p.title, &body, start.elapsed().as_micros()).into_response()
        }
        None => (StatusCode::NOT_FOUND, Html("<h1>Listing not found</h1>".to_string())).into_response(),
    }
}

async fn agents_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let start = std::time::Instant::now();
    let agents = state.agents.read().await;

    let mut body = String::from(r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">Our Agents</h1>"#);

    for a in agents.iter() {
        let stars = "★".repeat(a.rating as usize) + &"☆".repeat(5 - a.rating as usize);
        body.push_str(&format!(
            r#"<div class="agent-card" style="margin-bottom:1.25rem">
            <div class="avatar" style="font-size:4rem">{avatar}</div>
            <div style="flex:1">
                <h3 style="color:#f0f6fc;font-size:1.3rem">{name}</h3>
                <div class="title" style="margin-bottom:0.5rem">{title} · {years} years experience</div>
                <p style="margin:0.75rem 0;line-height:1.5">{bio}</p>
                <div class="features" style="margin:0.5rem 0">{specs}</div>
                <div style="display:flex;gap:2rem;margin-top:0.75rem;color:#8b949e">
                    <span>📋 {listings} active</span><span>🏠 {sold} sold</span>
                    <span class="stars">{stars} ({reviews})</span>
                </div>
                <div style="margin-top:0.75rem">{phone} · {email}</div>
            </div></div>"#,
            avatar = a.avatar, name = a.name, title = a.title, years = a.years_experience,
            bio = a.bio, listings = a.listings, sold = a.sold, reviews = a.reviews,
            phone = a.phone, email = a.email,
            specs = a.specialties.iter().map(|s| format!(r#"<span class="feature">{s}</span>"#)).collect::<String>(),
        ));
    }
    body.push_str("</div>");

    wrap("Agents", &body, start.elapsed().as_micros())
}

#[derive(Deserialize)]
struct MortgageQuery {
    price: Option<u64>,
    down: Option<f64>,
    rate: Option<f64>,
    term: Option<u32>,
}

async fn calculator(Query(q): Query<MortgageQuery>) -> Html<String> {
    let start = std::time::Instant::now();
    let price = q.price.unwrap_or(500000);
    let down_pct = q.down.unwrap_or(20.0);
    let rate = q.rate.unwrap_or(6.5);
    let term = q.term.unwrap_or(30);

    let down = price as f64 * (down_pct / 100.0);
    let loan = price as f64 - down;
    let monthly_rate = (rate / 100.0) / 12.0;
    let n = (term * 12) as f64;
    let payment = if monthly_rate > 0.0 {
        loan * (monthly_rate * (1.0 + monthly_rate).powf(n)) / ((1.0 + monthly_rate).powf(n) - 1.0)
    } else { loan / n };
    let total_paid = payment * n;
    let total_interest = total_paid - loan;

    let body = format!(
        r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">Mortgage Calculator</h1>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:2rem">
            <div class="card" style="padding:1.5rem">
                <form method="GET" action="/calculator">
                    <div class="form-group"><label>Home Price ($)</label><input type="number" name="price" value="{price}"></div>
                    <div class="form-group"><label>Down Payment (%)</label><input type="number" name="down" value="{down_pct}" step="0.5"></div>
                    <div class="form-group"><label>Interest Rate (%)</label><input type="number" name="rate" value="{rate}" step="0.125"></div>
                    <div class="form-group"><label>Loan Term (years)</label>
                        <select name="term"><option value="15" {sel15}>15 years</option><option value="20" {sel20}>20 years</option><option value="30" {sel30}>30 years</option></select>
                    </div>
                    <button class="btn" style="width:100%">Calculate</button>
                </form>
            </div>
            <div>
                <div class="stat-card" style="margin-bottom:1rem;padding:2rem">
                    <div class="label">Monthly Payment</div>
                    <div class="number" style="color:#3fb950;font-size:2.5rem">${payment:.0}</div>
                </div>
                <div class="stats-row" style="grid-template-columns:1fr 1fr;margin:0">
                    <div class="stat-card"><div class="number" style="font-size:1.3rem">{down_fmt}</div><div class="label">Down Payment</div></div>
                    <div class="stat-card"><div class="number" style="font-size:1.3rem">{loan_fmt}</div><div class="label">Loan Amount</div></div>
                    <div class="stat-card"><div class="number" style="font-size:1.3rem;color:#da3633">{interest_fmt}</div><div class="label">Total Interest</div></div>
                    <div class="stat-card"><div class="number" style="font-size:1.3rem">{total_fmt}</div><div class="label">Total Paid</div></div>
                </div>
            </div>
        </div></div>"#,
        down_pct = down_pct, rate = rate,
        sel15 = if term == 15 { "selected" } else { "" },
        sel20 = if term == 20 { "selected" } else { "" },
        sel30 = if term == 30 { "selected" } else { "" },
        payment = payment,
        down_fmt = format_price(down as u64),
        loan_fmt = format_price(loan as u64),
        interest_fmt = format_price(total_interest as u64),
        total_fmt = format_price(total_paid as u64),
    );

    wrap("Mortgage Calculator", &body, start.elapsed().as_micros())
}

#[derive(Deserialize)]
struct InquiryForm {
    name: String,
    email: String,
    phone: Option<String>,
    message: Option<String>,
}

async fn submit_inquiry(State(state): State<Arc<AppState>>, Path(id): Path<u32>, axum::Form(form): axum::Form<InquiryForm>) -> Redirect {
    let mut inquiries = state.inquiries.write().await;
    let mut next_id = state.next_inquiry_id.write().await;
    inquiries.push(Inquiry {
        id: *next_id,
        property_id: id,
        name: form.name,
        email: form.email,
        phone: form.phone.unwrap_or_default(),
        message: form.message.unwrap_or_default(),
        created_at: chrono_now(),
    });
    *next_id += 1;
    Redirect::to("/inquiries?success=1")
}

#[derive(Deserialize)]
struct InquiryQuery {
    success: Option<u8>,
}

async fn view_inquiries(State(state): State<Arc<AppState>>, Query(q): Query<InquiryQuery>) -> Html<String> {
    let start = std::time::Instant::now();
    let inquiries = state.inquiries.read().await;
    let properties = state.properties.read().await;

    let mut body = String::from(r#"<div class="container"><h1 style="color:#f0f6fc;margin-bottom:1.5rem">Inquiries</h1>"#);

    if q.success == Some(1) {
        body.push_str(r#"<div class="success-msg">Your inquiry has been submitted! An agent will contact you shortly.</div>"#);
    }

    if inquiries.is_empty() {
        body.push_str(r#"<div class="card" style="padding:2rem;text-align:center"><p>No inquiries yet</p></div>"#);
    } else {
        body.push_str("<table><thead><tr><th>Date</th><th>Property</th><th>Name</th><th>Email</th><th>Phone</th><th>Message</th></tr></thead><tbody>");
        for inq in inquiries.iter().rev() {
            let prop = properties.iter().find(|p| p.id == inq.property_id).map(|p| p.title.as_str()).unwrap_or("Unknown");
            body.push_str(&format!(
                "<tr><td>{}</td><td><a href=\"/listing/{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                inq.created_at, inq.property_id, prop, inq.name, inq.email, inq.phone, inq.message
            ));
        }
        body.push_str("</tbody></table>");
    }
    body.push_str("</div>");

    wrap("Inquiries", &body, start.elapsed().as_micros())
}

async fn api_listings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let properties = state.properties.read().await;
    let data: Vec<Value> = properties.iter().map(|p| json!({
        "id": p.id, "title": p.title, "price": p.price, "beds": p.beds, "baths": p.baths,
        "sqft": p.sqft, "city": p.city, "status": p.status, "type": p.property_type,
        "price_per_sqft": p.price_per_sqft, "days_on_market": p.days_on_market,
    })).collect();
    let elapsed = start.elapsed();
    axum::Json(json!({ "listings": data, "count": data.len(), "latency_us": elapsed.as_micros() }))
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
    let properties = seed_properties();
    let agents = seed_agents();
    let active = properties.iter().filter(|p| p.status == "Active").count();
    eprintln!("realestate ready: {} listings ({} active), {} agents", properties.len(), active, agents.len());

    let state = Arc::new(AppState {
        properties: RwLock::new(properties),
        agents: RwLock::new(agents),
        inquiries: RwLock::new(Vec::new()),
        next_inquiry_id: RwLock::new(1),
    });

    let app = Router::new()
        .route("/", get(listings))
        .route("/listing/{id}", get(listing_detail))
        .route("/agents", get(agents_page))
        .route("/calculator", get(calculator))
        .route("/inquiry/{id}", post(submit_inquiry))
        .route("/inquiries", get(view_inquiries))
        .route("/api/listings", get(api_listings))
        .with_state(state);

    let addr = "0.0.0.0:4003";
    eprintln!("realestate listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
