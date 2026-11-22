//! A document's `app`: what makes its pages an installable app. The page's head links the
//! manifest and registers the service worker; [`AppSpec::manifest`] is the manifest, and
//! [`AppSpec::worker`] what the service worker is told. `irender app` writes both beside
//! the rendered pages.

use crate::Diagnostics;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

/// An installable app.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AppSpec {
    /// The app's name.
    pub name: String,
    /// Under its icon, about 12 letters (default: the name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Where the app's files are, as the page finds them: `./` (default), `../` or `/app/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// The page it opens on, from `base` (default `./`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Display>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
    /// The browser's bar and the task switcher, a CSS colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    /// In dark mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color_dark: Option<String>,
    /// Behind the icon as it opens, a CSS colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    /// Icon files: `{ src, sizes, type, purpose }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<AppIcon>,
    /// Or an icon made from a letter or two: `{ text, color, background }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<MadeIcon>,
    /// In the icon's menu: `{ name, url, description }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shortcuts: Vec<Shortcut>,
    /// Pictures for the install dialog: `{ src, sizes, label }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Screenshot>,
    /// e.g. `[business, productivity]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    /// Takes links, text and files shared from other apps, for `ui-share-target`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<ShareTarget>,
    /// Files it opens, for `ui-file-handler`: `{ accept: { "text/csv": [".csv"] }, url }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offline: Option<Offline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Display {
    Standalone,
    Fullscreen,
    MinimalUi,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
    Any,
    Natural,
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppIcon {
    pub src: String,
    /// e.g. `512x512`, or `any` for an SVG.
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// `any`, `maskable` or `monochrome`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MadeIcon {
    /// One to three letters or digits.
    pub text: String,
    /// `#rrggbb`, of the letters (default white).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// `#rrggbb`, behind them (default near black).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Shortcut {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Screenshot {
    pub src: String,
    pub sizes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShareTarget {
    /// The page that takes them, from `base`.
    pub url: String,
    /// Kinds of files, e.g. `["image/*", "application/pdf"]` (default none: links and text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accept: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileHandler {
    /// The page that opens them, from `base`.
    pub url: String,
    /// Types and their endings.
    pub accept: IndexMap<String, Vec<String>>,
}

/// What's kept for when there's no connection.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Offline {
    /// Endings of the files kept from the start (default html, css, js, mjs, json, svg, png,
    /// jpg, webp, woff2, wasm and csv; `[]` keeps only what's visited).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// Files over this many KB are left until they're asked for (default 2048).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_kb: Option<u64>,
    /// The page shown when one that isn't kept is asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
    /// Paths that always go to the network, e.g. `["/api/"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skip: Vec<String>,
    /// Addresses on other sites to keep too, e.g. a library's. Those the pages load from
    /// jsDelivr are found by `irender app`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
}

/// A path within the app or an http(s) address, with nothing that could end a tag.
fn safe(url: &str) -> bool {
    !url.contains(['"', '<', '>', '\n', '\\']) && !url.trim_start().to_ascii_lowercase().starts_with("javascript:") && !url.contains("data:")
}

fn colour(c: &str) -> bool {
    !c.is_empty() && c.len() <= 64 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || "#(),.% /-".contains(ch))
}

fn hex(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

impl AppSpec {
    /// Reports what's wrong with it.
    pub fn check(&self, diags: &mut Diagnostics) {
        if self.name.trim().is_empty() {
            diags.push("app.name", "an app needs a name");
        }
        for (at, c) in [("themeColor", &self.theme_color), ("themeColorDark", &self.theme_color_dark), ("backgroundColor", &self.background_color)] {
            if let Some(c) = c
                && !colour(c)
            {
                diags.push(format!("app.{at}"), format!("`{c}` isn't a colour"));
            }
        }
        if let Some(b) = &self.base
            && (!safe(b) || !b.ends_with('/'))
        {
            diags.push("app.base", "`base` is a path that ends with `/`, such as `./` or `/app/`");
        }
        let urls = self
            .start_url
            .iter()
            .chain(self.icons.iter().map(|i| &i.src))
            .chain(self.shortcuts.iter().map(|s| &s.url))
            .chain(self.screenshots.iter().map(|s| &s.src))
            .chain(self.share.iter().map(|s| &s.url))
            .chain(self.files.iter().map(|f| &f.url))
            .chain(self.offline.iter().flat_map(|o| o.fallback.iter().chain(&o.skip).chain(&o.also)));
        for u in urls {
            if !safe(u) {
                diags.push("app", format!("`{u}` isn't an address an app can use"));
            }
        }
        if let Some(i) = &self.icon {
            let n = i.text.chars().count();
            if n == 0 || n > 3 || !i.text.chars().all(|c| c.is_ascii_alphanumeric()) {
                diags.push("app.icon.text", "an icon is made from one to three letters or digits; give `icons` for anything else");
            }
            for c in i.color.iter().chain(&i.background) {
                if !hex(c) {
                    diags.push("app.icon", format!("`{c}` isn't a colour like `#2563eb`"));
                }
            }
        }
        if let Some(a) = self.offline.iter().flat_map(|o| &o.also).find(|a| !a.starts_with("https://")) {
            diags.push("app.offline.also", format!("`{a}` must be an https address"));
        }
        for f in &self.files {
            if f.accept.is_empty() || f.accept.values().flatten().any(|e| !e.starts_with('.')) {
                diags.push("app.files", "each of `files` needs `accept`, a type and its endings: `{ \"text/csv\": [\".csv\"] }`");
            }
        }
    }

    pub fn base(&self) -> &str {
        self.base.as_deref().unwrap_or("./")
    }

    /// Its icons: those given, else the ones [`MadeIcon`] makes.
    fn icon_list(&self) -> Vec<Json> {
        if self.icons.is_empty() {
            return [("icon-192.png", "192x192", "any"), ("icon-512.png", "512x512", "any"), ("icon-maskable-512.png", "512x512", "maskable")]
                .iter()
                .map(|(src, sizes, purpose)| json!({ "src": format!("icons/{src}"), "sizes": sizes, "type": "image/png", "purpose": purpose }))
                .collect();
        }
        self.icons.iter().map(|i| serde_json::to_value(i).expect("plain")).collect()
    }

    /// The icon for an iPhone's home screen: the largest PNG.
    pub fn touch_icon(&self) -> String {
        if self.icons.is_empty() {
            return "icons/icon-180.png".into();
        }
        let size = |i: &AppIcon| i.sizes.split(['x', ' ']).next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(0);
        self.icons.iter().filter(|i| i.src.ends_with(".png")).max_by_key(|i| size(i)).unwrap_or(&self.icons[0]).src.clone()
    }

    /// The web app manifest.
    pub fn manifest(&self, lang: Option<&str>, dir: &str) -> Json {
        let mut m = json!({
            "name": self.name,
            "short_name": self.short_name.as_deref().unwrap_or(&self.name),
            "id": self.start_url.as_deref().unwrap_or("./"),
            "start_url": self.start_url.as_deref().unwrap_or("./"),
            "scope": "./",
            "display": self.display.unwrap_or(Display::Standalone),
            "icons": self.icon_list(),
            "dir": dir,
        });
        let mut put = |k: &str, v: Option<Json>| {
            if let Some(v) = v {
                m[k] = v;
            }
        };
        put("lang", lang.map(|l| json!(l)));
        put("description", self.description.as_ref().map(|d| json!(d)));
        put("theme_color", self.theme_color.as_ref().map(|c| json!(c)));
        put("background_color", self.background_color.as_ref().map(|c| json!(c)));
        put("orientation", self.orientation.map(|o| json!(o)));
        put("categories", (!self.categories.is_empty()).then(|| json!(self.categories)));
        put("shortcuts", (!self.shortcuts.is_empty()).then(|| json!(self.shortcuts)));
        put("screenshots", (!self.screenshots.is_empty()).then(|| json!(self.screenshots)));
        put(
            "share_target",
            self.share.as_ref().map(|s| {
                let mut params = json!({ "title": "title", "text": "text", "url": "url" });
                if !s.accept.is_empty() {
                    params["files"] = json!([{ "name": "files", "accept": s.accept }]);
                }
                json!({ "action": s.url, "method": "POST", "enctype": "multipart/form-data", "params": params })
            }),
        );
        put("file_handlers", (!self.files.is_empty()).then(|| json!(self.files.iter().map(|f| json!({ "action": f.url, "accept": f.accept })).collect::<Vec<_>>())));
        m
    }

    /// What the service worker is told: the files to keep, and how.
    pub fn worker(&self, version: &str, files: &[String], also: &[String]) -> Json {
        let offline = self.offline.clone().unwrap_or_default();
        json!({
            "version": version, "files": files, "also": also, "fallback": offline.fallback, "skip": offline.skip,
            "share": self.share.as_ref().map(|s| &s.url), "start": self.start_url.as_deref().unwrap_or("./"),
            "name": self.name, "icon": self.icon_list().first().map(|i| i["src"].clone()),
        })
    }

    /// The tags for a page's head: the manifest, its colours, an iPhone's icon and title.
    pub fn head(&self) -> String {
        let base = self.base();
        let esc = |s: &str| s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
        let mut s = format!("<link rel=\"manifest\" href=\"{}manifest.webmanifest\">\n", esc(base));
        match (&self.theme_color, &self.theme_color_dark) {
            (Some(l), Some(d)) => s += &format!(
                "<meta name=\"theme-color\" content=\"{}\" media=\"(prefers-color-scheme: light)\">\n<meta name=\"theme-color\" content=\"{}\" media=\"(prefers-color-scheme: dark)\">\n",
                esc(l),
                esc(d)
            ),
            (Some(c), None) | (None, Some(c)) => s += &format!("<meta name=\"theme-color\" content=\"{}\">\n", esc(c)),
            (None, None) => {}
        }
        let icon = self.touch_icon();
        let icon = if icon.starts_with('/') || icon.contains("://") { icon } else { format!("{base}{icon}") };
        s += &format!("<link rel=\"apple-touch-icon\" href=\"{}\">\n", esc(&icon));
        s += &format!("<link rel=\"icon\" href=\"{}\">\n", esc(&icon));
        s += "<meta name=\"mobile-web-app-capable\" content=\"yes\">\n<meta name=\"apple-mobile-web-app-capable\" content=\"yes\">\n<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"default\">\n";
        s += &format!("<meta name=\"apple-mobile-web-app-title\" content=\"{}\">\n", esc(self.short_name.as_deref().unwrap_or(&self.name)));
        // A page that can't (an old browser, a file) goes on without it.
        let base = serde_json::to_string(base).expect("a string").replace("</", "<\\/");
        s += &format!(
            "<script>if (\"serviceWorker\" in navigator && location.protocol !== \"file:\") addEventListener(\"load\", () => navigator.serviceWorker.register({base} + \"sw.js\", {{ scope: {base} }}).catch(() => {{}}));</script>\n"
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(v: Json) -> AppSpec {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn manifest_and_head_follow_the_settings() {
        let a = app(json!({ "name": "Field \"app\"", "themeColor": "#111", "base": "/app/", "share": { "url": "./in.html", "accept": ["image/*"] }, "files": [{ "url": "./open.html", "accept": { "text/csv": [".csv"] } }] }));
        let m = a.manifest(Some("en"), "ltr");
        assert_eq!(m["display"], "standalone");
        assert_eq!(m["short_name"], "Field \"app\"");
        assert_eq!(m["icons"].as_array().unwrap().len(), 3);
        assert_eq!(m["share_target"]["params"]["files"][0]["accept"], json!(["image/*"]));
        assert_eq!(m["file_handlers"][0]["accept"]["text/csv"], json!([".csv"]));
        let h = a.head();
        assert!(h.contains(r#"<link rel="manifest" href="/app/manifest.webmanifest">"#), "{h}");
        assert!(h.contains(r#"content="Field &quot;app&quot;""#), "{h}");
        assert!(h.contains(r#"register("/app/" + "sw.js""#), "{h}");
    }

    #[test]
    fn what_cant_be_used_is_reported() {
        let mut d = Diagnostics::default();
        app(json!({ "name": " ", "base": "app", "themeColor": "red;x", "icon": { "text": "ABCD", "background": "red" }, "shortcuts": [{ "name": "x", "url": "javascript:1" }], "offline": { "also": ["http://x"] } })).check(&mut d);
        let text = d.to_string();
        for part in ["needs a name", "ends with `/`", "isn't a colour", "one to three letters", "isn't an address", "must be an https"] {
            assert!(text.contains(part), "{part}: {text}");
        }
    }
}
