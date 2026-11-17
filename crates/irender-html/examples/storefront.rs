//! A host app whose UI config is partly generated at runtime.
//!
//! `examples/storefront/storefront.json` is the fixed layout. Three of its parts are
//! `include`s that this program builds from "API" responses:
//!
//! - `nav`: one link per category from the catalog service
//! - `hero`: the current campaign, whose *structure* depends on the A/B variant, or
//!   nothing when no campaign is running
//! - `recommendations`: present only when the feature flag is on
//!
//! Per-request values (the user and their products) go in as server `data`. The cart
//! lives in the browser: buttons `call` the host script below, which updates `state`.
//!
//!   cargo run -p irender-html --example storefront > storefront.html
//!   cargo run -p irender-html --example storefront -- --variant b --no-campaign --no-recs > storefront.html

use irender_core::{Format, Fragments, NodeSpec};
use irender_html::{Engine, Page};
use serde_json::{Value as Json, json};
use std::{error::Error, sync::Arc};

/// Stand-ins for the services a real host would call.
mod api {
    use serde_json::{Value as Json, json};

    pub struct Campaign {
        pub headline: &'static str,
        pub body: &'static str,
        pub cta: &'static str,
        pub url: &'static str,
        /// The A/B test arm.
        pub variant: char,
    }

    pub fn categories() -> Vec<(&'static str, &'static str)> {
        vec![("Coffee", "/c/coffee"), ("Cookware", "/c/cookware"), ("Outdoor", "/c/outdoor"), ("Sale", "/c/sale")]
    }

    pub fn campaign(variant: char, running: bool) -> Option<Campaign> {
        running.then_some(Campaign {
            headline: "Monsoon brew week",
            body: "20% off every pour-over kit until Sunday.",
            cta: "Shop the kits",
            url: "/c/coffee?promo=monsoon",
            variant,
        })
    }

    pub fn recommendations() -> Json {
        json!([{ "name": "Filter Papers (100)", "price": 299 }, { "name": "Scoop", "price": 300 }, { "name": "Scale", "price": 1450 }])
    }

    /// The per-request part: who is looking and what they see.
    pub fn request_data() -> Json {
        json!({
            "user": { "name": "Ravi", "tier": "gold" },
            "products": [
                { "id": "k1", "name": "Pour-Over Set", "price": 1499, "stock": 12 },
                { "id": "k2", "name": "Burr Grinder", "price": 4200, "stock": 3 },
                { "id": "k3", "name": "Gooseneck Kettle", "price": 2650, "stock": 0 },
                { "id": "o2", "name": "Insulated Flask", "price": 1199, "stock": 8 }
            ]
        })
    }
}

/// Builds the generated parts of the config. Each fragment is plain JSON that is then
/// validated like the rest of the document, so a bad API response becomes a located
/// error (`root.children[1]@hero…`) rather than broken HTML.
struct Generated {
    categories: Vec<(&'static str, &'static str)>,
    campaign: Option<api::Campaign>,
    recommendations: Option<Json>,
}

impl Generated {
    fn nav(&self) -> Json {
        let links: Vec<Json> = self
            .categories
            .iter()
            .map(|(label, href)| {
                json!({ "type": "text", "props": { "tag": "a", "value": label }, "attrs": { "href": href },
                        "style": { "color": "$color.text", "text-decoration": "none", ":hover": { "color": "$color.primary" } } })
            })
            .collect();
        json!({ "type": "stack", "props": { "tag": "nav", "direction": "row", "gap": "$space.md" }, "children": links })
    }

    fn hero(&self) -> Json {
        let Some(c) = &self.campaign else {
            // No campaign: an empty, invisible node keeps the layout's include valid.
            return json!({ "type": "element", "attrs": { "hidden": true } });
        };
        let text = json!({ "type": "stack", "props": { "gap": "$space.xs" }, "children": [
            { "type": "text", "props": { "tag": "h1", "value": c.headline }, "style": { "margin": 0 } },
            { "type": "text", "props": { "tag": "p", "value": c.body }, "style": { "margin": 0 } }
        ] });
        let cta = json!({ "type": "button", "props": { "label": c.cta },
                          "on": { "click": { "action": "navigate", "url": c.url } } });
        match c.variant {
            // Variant B: a compact banner with the button beside the text.
            'b' => json!({ "type": "stack", "props": { "direction": "row", "justify": "space-between", "align": "center", "gap": "$space.md" },
                           "attrs": { "data-variant": "b" },
                           "style": { "padding": "$space.md $space.lg", "border-radius": "$radius.lg", "background": "#ffedd5" },
                           "children": [text, cta] }),
            // Variant A: a tall hero with the button underneath.
            _ => json!({ "type": "stack", "props": { "gap": "$space.md", "align": "flex-start" },
                         "attrs": { "data-variant": "a" },
                         "style": { "padding": "$space.xl", "border-radius": "$radius.lg", "color": "#fff",
                                    "background": "linear-gradient(135deg, #c2410c, #ea580c)" },
                         "children": [text, cta] }),
        }
    }

    fn recommendations(&self) -> Json {
        let Some(items) = &self.recommendations else {
            return json!({ "type": "element", "attrs": { "hidden": true } });
        };
        // The API response is baked into the fragment as a literal list.
        json!({ "type": "stack", "props": { "gap": "$space.sm" }, "children": [
            { "type": "text", "props": { "tag": "h2", "value": "Often bought together" }, "style": { "margin": 0, "font-size": 18 } },
            { "type": "list", "props": { "items": items, "as": "r" }, "style": { "display": "flex", "gap": "$space.sm", "flex-wrap": "wrap" },
              "children": [{ "type": "text", "props": { "value": "{{ r.name }} · ₹{{ r.price }}" },
                             "style": { "padding": "6px 12px", "border": "1px solid $color.border", "border-radius": 999 } }] }
        ] })
    }
}

impl Fragments for Generated {
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String> {
        let node = match name {
            "nav" => self.nav(),
            "hero" => self.hero(),
            "recommendations" => self.recommendations(),
            _ => return Ok(None),
        };
        serde_json::from_value(node).map(Some).map_err(|e| e.to_string())
    }
}

/// The cart. The config only says *when* to add or remove (`call` actions); the host
/// owns the logic and writes the results back into state.
const HOST_SCRIPT: &str = r#"<script>
const root = document.querySelector("[data-i-root]");
let cart = [];
root.addEventListener("irender:call", (e) => {
  const { name, args } = e.detail;
  if (name === "add") {
    const line = cart.find((l) => l.id === args.id);
    cart = line ? cart.map((l) => (l === line ? { ...l, qty: l.qty + 1 } : l)) : [...cart, { ...args, qty: 1 }];
  } else if (name === "remove") {
    cart = cart.filter((l) => l.id !== args.id);
  } else if (name === "checkout") {
    console.log("checkout", args);
    return;
  } else {
    return;
  }
  // A new array each time: lists rebuild when their array changes.
  root.__irender.set("count", cart.reduce((n, l) => n + l.qty, 0));
  root.__irender.set("total", cart.reduce((n, l) => n + l.qty * l.price, 0));
  root.__irender.set("cart", cart);
});
</script>"#;

fn layout(page: &Page) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Storefront</title>\n</head>\n<body style=\"margin:0\">\n{}{HOST_SCRIPT}\n</body>\n</html>\n",
        page.fragment()
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let variant = if args.windows(2).any(|w| w[0] == "--variant" && w[1] == "b") { 'b' } else { 'a' };

    // Load time: call the services and build the generated fragments. A real host does
    // this when the campaign, catalog or flags change, and caches the resulting `Ir`.
    let mut engine = Engine::new();
    engine.fragments = Some(Arc::new(Generated {
        categories: api::categories(),
        campaign: api::campaign(variant, !has("--no-campaign")),
        recommendations: (!has("--no-recs")).then(api::recommendations),
    }));
    let src = include_str!("../../../examples/storefront/storefront.json");
    let ir = engine.load(src, Format::Json)?;
    for w in ir.warnings.iter() {
        eprintln!("{w}");
    }

    // Request time: render with this visitor's data.
    let page = engine.render(&ir, &api::request_data())?;
    print!("{}", layout(&page));
    Ok(())
}
