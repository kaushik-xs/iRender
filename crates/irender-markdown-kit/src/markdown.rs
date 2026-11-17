//! A small, safe Markdown renderer: a CommonMark subset with GitHub's tables, task
//! lists, strikethrough and bare links. `markdown.js` is the same renderer for the
//! browser, line for line, so the live preview matches what the server renders; the
//! parity test holds them to it.
//!
//! Safe by construction: every piece of text is escaped, raw HTML shows as text, and
//! a link or image whose URL isn't `http(s):`, `mailto:`, a `#fragment` or relative
//! renders as its text alone.

/// Renders Markdown to HTML.
pub fn render(src: &str) -> String {
    let text = src.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ");
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    blocks(&lines, false)
}

// ---------------------------------------------------------------- blocks

fn blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// A code fence: up to three spaces, then three or more backticks or tildes.
fn fence(line: &str) -> Option<(char, usize, String)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let c = t.chars().next()?;
    if c != '`' && c != '~' {
        return None;
    }
    let n = t.chars().take_while(|&x| x == c).count();
    if n < 3 {
        return None;
    }
    let info = t[n..].trim();
    if c == '`' && info.contains('`') {
        return None;
    }
    Some((c, n, info.split(' ').next().unwrap_or("").to_string()))
}

fn heading(line: &str) -> Option<(usize, String)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let level = t.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &t[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let mut text = rest.trim();
    // A closing run of `#`, after a space.
    let closing = text.trim_end_matches('#');
    if closing.is_empty() {
        text = "";
    } else if closing.ends_with(' ') {
        text = closing.trim_end();
    }
    Some((level, text.to_string()))
}

fn rule(line: &str) -> bool {
    if indent(line) > 3 {
        return false;
    }
    let t: String = line.chars().filter(|&c| c != ' ').collect();
    t.len() >= 3 && ['-', '*', '_'].iter().any(|&c| t.chars().all(|x| x == c))
}

fn quote(line: &str) -> bool {
    indent(line) <= 3 && line.trim_start().starts_with('>')
}

/// A list item's marker: (indent, ordered, number, the column its content starts at).
struct Marker {
    indent: usize,
    ordered: bool,
    number: u64,
    content: usize,
}

fn marker(line: &str) -> Option<Marker> {
    let ind = indent(line);
    let t = &line[ind..];
    let (ordered, number, len) = if t.starts_with(['-', '*', '+']) {
        (false, 0, 1)
    } else {
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        match t[digits..].chars().next() {
            Some('.') | Some(')') => (true, t[..digits].parse().unwrap_or(1), digits + 1),
            _ => return None,
        }
    };
    let after = &t[len..];
    if after.is_empty() {
        return Some(Marker { indent: ind, ordered, number, content: ind + len + 1 });
    }
    if !after.starts_with(' ') {
        return None;
    }
    let spaces = after.len() - after.trim_start_matches(' ').len();
    // Five or more spaces: the content is indented code; keep one as the gap.
    let gap = if spaces > 4 || after.trim().is_empty() { 1 } else { spaces };
    Some(Marker { indent: ind, ordered, number, content: ind + len + gap })
}

/// The text after a marker, from its content column.
fn after_marker(line: &str, m: &Marker) -> String {
    line.get(m.content..).unwrap_or("").to_string()
}

fn table_row(line: &str) -> bool {
    line.contains('|')
}

fn delimiter_row(line: &str) -> Option<Vec<&'static str>> {
    let cells = split_row(line);
    if cells.is_empty() {
        return None;
    }
    let mut aligns = Vec::new();
    for c in cells {
        let c = c.trim();
        let left = c.starts_with(':');
        let right = c.ends_with(':');
        let dashes = c.trim_matches(':');
        if dashes.is_empty() || !dashes.chars().all(|x| x == '-') {
            return None;
        }
        aligns.push(match (left, right) {
            (true, true) => "center",
            (true, false) => "left",
            (false, true) => "right",
            _ => "",
        });
    }
    Some(aligns)
}

/// A table row's cells: split on unescaped `|`, outer pipes dropped.
fn split_row(line: &str) -> Vec<String> {
    let mut t = line.trim();
    if let Some(s) = t.strip_prefix('|') {
        t = s;
    }
    if t.ends_with('|') && !t.ends_with("\\|") {
        t = &t[..t.len() - 1];
    }
    let mut cells = vec![String::new()];
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            cells.last_mut().unwrap().push('|');
            chars.next();
        } else if c == '|' {
            cells.push(String::new());
        } else {
            cells.last_mut().unwrap().push(c);
        }
    }
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

/// Whether `line` starts a block that ends a paragraph.
fn interrupts(line: &str) -> bool {
    fence(line).is_some() || heading(line).is_some() || rule(line) || quote(line) || marker(line).is_some_and(|m| !m.ordered || m.number == 1)
}

fn blocks(lines: &[String], tight: bool) -> String {
    let mut out: Vec<String> = Vec::new();
    let n = lines.len();
    let mut i = 0;
    while i < n {
        let line = &lines[i];
        if blank(line) {
            i += 1;
            continue;
        }
        if let Some((c, len, lang)) = fence(line) {
            let ind = indent(line);
            i += 1;
            let mut code = Vec::new();
            while i < n {
                let l = &lines[i];
                if fence(l).is_some_and(|(c2, len2, info)| c2 == c && len2 >= len && info.is_empty()) {
                    i += 1;
                    break;
                }
                // Lines lose up to the fence's own indent.
                let strip = indent(l).min(ind);
                code.push(l[strip..].to_string());
                i += 1;
            }
            let class = if !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphanumeric() || "_+-.#".contains(c)) {
                format!(" class=\"language-{}\"", escape(&lang))
            } else {
                String::new()
            };
            out.push(format!("<pre><code{class}>{}</code></pre>", escape(&code.join("\n"))));
            continue;
        }
        if let Some((level, text)) = heading(line) {
            out.push(format!("<h{level}>{}</h{level}>", inline(&text)));
            i += 1;
            continue;
        }
        if rule(line) {
            out.push("<hr>".into());
            i += 1;
            continue;
        }
        if quote(line) {
            let mut inner = Vec::new();
            while i < n && quote(&lines[i]) {
                let t = lines[i].trim_start();
                let t = &t[1..];
                inner.push(t.strip_prefix(' ').unwrap_or(t).to_string());
                i += 1;
            }
            out.push(format!("<blockquote>\n{}\n</blockquote>", blocks(&inner, false)));
            continue;
        }
        if let Some(m) = marker(line) {
            let (html, next) = list(lines, i, m);
            out.push(html);
            i = next;
            continue;
        }
        if table_row(line) && i + 1 < n && let Some(aligns) = delimiter_row(&lines[i + 1]) {
            let head = split_row(line);
            if head.len() == aligns.len() {
                let cell = |tag: &str, i: usize, text: &str| {
                    let align = aligns.get(i).copied().unwrap_or("");
                    let style = if align.is_empty() { String::new() } else { format!(" style=\"text-align: {align}\"") };
                    format!("<{tag}{style}>{}</{tag}>", inline(text))
                };
                let mut html = String::from("<table>\n<thead>\n<tr>");
                for (k, c) in head.iter().enumerate() {
                    html.push_str(&cell("th", k, c));
                }
                html.push_str("</tr>\n</thead>");
                i += 2;
                let mut rows = Vec::new();
                while i < n && !blank(&lines[i]) && table_row(&lines[i]) {
                    let cells = split_row(&lines[i]);
                    let mut row = String::from("<tr>");
                    for k in 0..aligns.len() {
                        row.push_str(&cell("td", k, cells.get(k).map_or("", String::as_str)));
                    }
                    row.push_str("</tr>");
                    rows.push(row);
                    i += 1;
                }
                if !rows.is_empty() {
                    html.push_str("\n<tbody>\n");
                    html.push_str(&rows.join("\n"));
                    html.push_str("\n</tbody>");
                }
                html.push_str("\n</table>");
                out.push(html);
                continue;
            }
        }
        // A paragraph: lines up to a blank one or the start of another block.
        let mut para = vec![line.trim_start().to_string()];
        i += 1;
        while i < n && !blank(&lines[i]) && !interrupts(&lines[i]) {
            para.push(lines[i].trim_start().to_string());
            i += 1;
        }
        let text = inline(para.join("\n").trim_end());
        out.push(if tight { text } else { format!("<p>{text}</p>") });
    }
    out.join("\n")
}

/// A list starting at `lines[start]`, and the index of the line after it.
fn list(lines: &[String], start: usize, first: Marker) -> (String, usize) {
    let n = lines.len();
    let ordered = first.ordered;
    let number = first.number;
    let mut items: Vec<Vec<String>> = Vec::new();
    let mut loose = false;
    let mut i = start;
    while i < n {
        let Some(m) = marker(&lines[i]).filter(|m| m.ordered == ordered && m.indent < first.content) else { break };
        let mut item = vec![after_marker(&lines[i], &m)];
        i += 1;
        while i < n {
            let l = &lines[i];
            if blank(l) {
                // A blank line stays in the item if the item goes on after it.
                let mut j = i;
                while j < n && blank(&lines[j]) {
                    j += 1;
                }
                if j < n && indent(&lines[j]) >= m.content {
                    for _ in i..j {
                        item.push(String::new());
                    }
                    loose = true;
                    i = j;
                    continue;
                }
                break;
            }
            if indent(l) >= m.content {
                item.push(l[m.content..].to_string());
                i += 1;
                continue;
            }
            if marker(l).is_some() || interrupts(l) {
                break;
            }
            // A lazy continuation of the item's paragraph.
            item.push(l.trim_start().to_string());
            i += 1;
        }
        items.push(item);
        // Blank lines between items make the list loose.
        let mut j = i;
        while j < n && blank(&lines[j]) {
            j += 1;
        }
        if j > i && j < n && marker(&lines[j]).is_some_and(|m2| m2.ordered == ordered && m2.indent < first.content) {
            loose = true;
            i = j;
        }
    }
    let tag = if ordered { "ol" } else { "ul" };
    let start_attr = if ordered && number != 1 { format!(" start=\"{number}\"") } else { String::new() };
    let mut html = format!("<{tag}{start_attr}>");
    for mut item in items {
        // A task: `[ ]` or `[x]` before the text.
        let mut prefix = String::new();
        let mut class = "";
        if let Some(first) = item.first_mut() {
            let (box_, rest) = (first.get(..4), first.get(4..));
            if let (Some(b), Some(rest)) = (box_, rest)
                && (b == "[ ] " || b == "[x] " || b == "[X] ")
            {
                let checked = if b == "[ ] " { "" } else { " checked" };
                prefix = format!("<input type=\"checkbox\" disabled{checked}> ");
                class = " class=\"task\"";
                *first = rest.to_string();
            }
        }
        let body = blocks(&item, !loose);
        // Loose items hold their paragraphs on lines of their own; a tight item with a
        // nested list ends that list on its own line.
        let body = if loose {
            format!("\n{body}\n")
        } else if body.contains('\n') {
            format!("{body}\n")
        } else {
            body
        };
        html.push_str(&format!("\n<li{class}>{prefix}{body}</li>"));
    }
    html.push_str(&format!("\n</{tag}>"));
    (html, i)
}

// ---------------------------------------------------------------- inline

pub(crate) fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A URL a link or image may use: `http(s):`, `mailto:`, a fragment, or relative.
fn safe_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:") {
        return true;
    }
    // Relative: no scheme before the first `/`, `?` or `#`.
    let head: String = u.chars().take_while(|c| !matches!(c, '/' | '?' | '#')).collect();
    !head.contains(':')
}

fn space(c: Option<char>) -> bool {
    c.is_none_or(|c| c == ' ' || c == '\n')
}

fn word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric())
}

/// The end of the code span opening at `i` (a run of backticks), if it closes.
fn code_end(s: &[char], i: usize) -> Option<(usize, usize)> {
    let n = s[i..].iter().take_while(|&&c| c == '`').count();
    let mut k = i + n;
    while k < s.len() {
        if s[k] == '`' {
            let m = s[k..].iter().take_while(|&&c| c == '`').count();
            if m == n {
                return Some((n, k));
            }
            k += m;
        } else {
            k += 1;
        }
    }
    None
}

/// The `]` that matches the `[` at `i`.
fn bracket_end(s: &[char], i: usize) -> Option<usize> {
    let mut depth = 0;
    let mut k = i;
    while k < s.len() {
        match s[k] {
            '\\' => k += 1,
            '`' => {
                if let Some((n, end)) = code_end(s, k) {
                    k = end + n - 1;
                }
            }
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// `(url "title")` from `i` (the `(`): the URL, the title and the index after `)`.
fn destination(s: &[char], i: usize) -> Option<(String, Option<String>, usize)> {
    if s.get(i) != Some(&'(') {
        return None;
    }
    let mut k = i + 1;
    while s.get(k) == Some(&' ') {
        k += 1;
    }
    let mut url = String::new();
    if s.get(k) == Some(&'<') {
        k += 1;
        while k < s.len() && s[k] != '>' && s[k] != '\n' {
            url.push(s[k]);
            k += 1;
        }
        if s.get(k) != Some(&'>') {
            return None;
        }
        k += 1;
    } else {
        let mut depth = 0;
        while k < s.len() {
            let c = s[k];
            if c == ' ' || c == '\n' {
                break;
            }
            if c == '\\' && k + 1 < s.len() && s[k + 1].is_ascii_punctuation() {
                url.push(s[k + 1]);
                k += 2;
                continue;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            url.push(c);
            k += 1;
        }
    }
    while s.get(k) == Some(&' ') {
        k += 1;
    }
    let mut title = None;
    if let Some(&q) = s.get(k).filter(|&&c| c == '"' || c == '\'') {
        let mut t = String::new();
        k += 1;
        while k < s.len() && s[k] != q {
            t.push(s[k]);
            k += 1;
        }
        if s.get(k) != Some(&q) {
            return None;
        }
        k += 1;
        title = Some(t);
        while s.get(k) == Some(&' ') {
            k += 1;
        }
    }
    if s.get(k) != Some(&')') {
        return None;
    }
    Some((url, title, k + 1))
}

/// The closing delimiter for a run of `n` × `c` opening at `i`.
fn closer(s: &[char], i: usize, c: char, n: usize) -> Option<usize> {
    let mut k = i + n;
    while k < s.len() {
        match s[k] {
            '\\' => k += 2,
            '`' => match code_end(s, k) {
                Some((m, end)) => k = end + m,
                None => k += 1,
            },
            x if x == c => {
                let run = s[k..].iter().take_while(|&&y| y == c).count();
                let before = s.get(k.wrapping_sub(1)).copied();
                let after = s.get(k + run).copied();
                if run == n && k > i + n && !space(before) && (c != '_' || !word(after)) {
                    return Some(k);
                }
                k += run;
            }
            _ => k += 1,
        }
    }
    None
}

/// Renders inline Markdown.
pub(crate) fn inline(text: &str) -> String {
    let s: Vec<char> = text.chars().collect();
    span(&s, 0, s.len())
}

fn span(s: &[char], from: usize, to: usize) -> String {
    let s = &s[..to];
    let mut out = String::new();
    let mut i = from;
    while i < to {
        let c = s[i];
        let next = s.get(i + 1).copied();
        // Backslash escapes, and a backslash before a line break.
        if c == '\\' {
            if next == Some('\n') {
                out.push_str("<br>\n");
                i += 2;
                continue;
            }
            if let Some(n) = next.filter(char::is_ascii_punctuation) {
                out.push_str(&escape(&n.to_string()));
                i += 2;
                continue;
            }
        }
        if c == '`' {
            match code_end(s, i) {
                Some((n, end)) => {
                    let mut code: String = s[i + n..end].iter().collect::<String>().replace('\n', " ");
                    if code.len() >= 2 && code.starts_with(' ') && code.ends_with(' ') && !code.trim().is_empty() {
                        code = code[1..code.len() - 1].to_string();
                    }
                    out.push_str(&format!("<code>{}</code>", escape(&code)));
                    i = end + n;
                }
                None => {
                    let n = s[i..].iter().take_while(|&&x| x == '`').count();
                    out.push_str(&"`".repeat(n));
                    i += n;
                }
            }
            continue;
        }
        // Images and links.
        let image = c == '!' && next == Some('[');
        if image || c == '[' {
            let open = if image { i + 1 } else { i };
            if let Some(close) = bracket_end(s, open)
                && let Some((url, title, end)) = destination(s, close + 1)
            {
                let label = &s[open + 1..close];
                let title = title.map_or(String::new(), |t| format!(" title=\"{}\"", escape(&t)));
                if image {
                    let alt: String = label.iter().collect();
                    out.push_str(&if safe_url(&url) {
                        format!("<img src=\"{}\" alt=\"{}\"{title}>", escape(url.trim()), escape(&alt))
                    } else {
                        escape(&alt)
                    });
                } else {
                    let inner = span(s, open + 1, close);
                    out.push_str(&if safe_url(&url) { format!("<a href=\"{}\"{title}>{inner}</a>", escape(url.trim())) } else { inner });
                }
                i = end;
                continue;
            }
        }
        // `<https://…>` and `<mailto:…>`.
        if c == '<' {
            let rest: String = s[i + 1..].iter().take_while(|&&x| x != '>' && x != ' ' && x != '\n' && x != '<').collect();
            let lower = rest.to_ascii_lowercase();
            if s.get(i + 1 + rest.chars().count()) == Some(&'>')
                && (lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:"))
            {
                let shown = rest.strip_prefix("mailto:").unwrap_or(&rest);
                out.push_str(&format!("<a href=\"{}\">{}</a>", escape(&rest), escape(shown)));
                i += rest.chars().count() + 2;
                continue;
            }
        }
        // Bare `http(s)://` links, without trailing punctuation.
        if (c == 'h' || c == 'H') && !word(i.checked_sub(1).and_then(|p| s.get(p).copied())) {
            let rest: String = s[i..].iter().take_while(|&&x| x != ' ' && x != '\n' && x != '<').collect();
            let lower = rest.to_ascii_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                let mut url: Vec<char> = rest.chars().collect();
                while url.last().is_some_and(|c| ".,:;!?\"')]*_~".contains(*c)) {
                    url.pop();
                }
                if url.len() > 8 {
                    let u: String = url.iter().collect();
                    out.push_str(&format!("<a href=\"{0}\">{0}</a>", escape(&u)));
                    i += url.len();
                    continue;
                }
            }
        }
        // Emphasis: `*em*`, `**strong**`, `***both***`, `_` the same, `~~strike~~`.
        if c == '*' || c == '_' || c == '~' {
            let n = s[i..].iter().take_while(|&&x| x == c).count();
            let valid = if c == '~' { n == 2 } else { n <= 3 };
            let prev = i.checked_sub(1).and_then(|p| s.get(p).copied());
            if valid && !space(s.get(i + n).copied()) && (c != '_' || !word(prev))
                && let Some(k) = closer(s, i, c, n)
            {
                let inner = span(s, i + n, k);
                out.push_str(&match (c, n) {
                    ('~', _) => format!("<del>{inner}</del>"),
                    (_, 1) => format!("<em>{inner}</em>"),
                    (_, 2) => format!("<strong>{inner}</strong>"),
                    _ => format!("<em><strong>{inner}</strong></em>"),
                });
                i = k + n;
                continue;
            }
            out.push_str(&c.to_string().repeat(n));
            i += n;
            continue;
        }
        // Two or more spaces before a line break make it a hard break.
        if c == '\n' {
            let trimmed = out.trim_end_matches(' ');
            let spaces = out.len() - trimmed.len();
            out.truncate(trimmed.len());
            out.push_str(if spaces >= 2 { "<br>\n" } else { "\n" });
            i += 1;
            continue;
        }
        out.push_str(&escape(&c.to_string()));
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn blocks() {
        assert_eq!(render("# Title\n\nSome *text*."), "<h1>Title</h1>\n<p>Some <em>text</em>.</p>");
        assert_eq!(render("- a\n- b"), "<ul>\n<li>a</li>\n<li>b</li>\n</ul>");
        assert_eq!(render("3. c\n4. d"), "<ol start=\"3\">\n<li>c</li>\n<li>d</li>\n</ol>");
        assert_eq!(render("```rust\nlet x = 1 < 2;\n```"), "<pre><code class=\"language-rust\">let x = 1 &lt; 2;</code></pre>");
        assert_eq!(render("> quoted\n> more"), "<blockquote>\n<p>quoted\nmore</p>\n</blockquote>");
        assert_eq!(render("---"), "<hr>");
        assert_eq!(render("- [x] done\n- [ ] todo"), "<ul>\n<li class=\"task\"><input type=\"checkbox\" disabled checked> done</li>\n<li class=\"task\"><input type=\"checkbox\" disabled> todo</li>\n</ul>");
    }

    #[test]
    fn nested_and_loose_lists() {
        assert_eq!(render("- a\n  - b\n- c"), "<ul>\n<li>a\n<ul>\n<li>b</li>\n</ul>\n</li>\n<li>c</li>\n</ul>");
        assert_eq!(render("- a\n\n- b"), "<ul>\n<li>\n<p>a</p>\n</li>\n<li>\n<p>b</p>\n</li>\n</ul>");
    }

    #[test]
    fn tables() {
        assert_eq!(
            render("| a | b |\n|:--|--:|\n| 1 | 2 |"),
            "<table>\n<thead>\n<tr><th style=\"text-align: left\">a</th><th style=\"text-align: right\">b</th></tr>\n</thead>\n<tbody>\n<tr><td style=\"text-align: left\">1</td><td style=\"text-align: right\">2</td></tr>\n</tbody>\n</table>"
        );
    }

    #[test]
    fn inline_markup() {
        assert_eq!(render("**b** _i_ ~~s~~ `c`"), "<p><strong>b</strong> <em>i</em> <del>s</del> <code>c</code></p>");
        assert_eq!(render("[x](https://a.b \"t\")"), "<p><a href=\"https://a.b\" title=\"t\">x</a></p>");
        assert_eq!(render("see https://a.b/c."), "<p>see <a href=\"https://a.b/c\">https://a.b/c</a>.</p>");
        assert_eq!(render("snake_case_name"), "<p>snake_case_name</p>");
        assert_eq!(render("line  \nbreak"), "<p>line<br>\nbreak</p>");
    }

    #[test]
    fn unsafe_input_stays_inert() {
        assert_eq!(render("<script>alert(1)</script>"), "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
        assert_eq!(render("[x](javascript:alert(1))"), "<p>x</p>");
        assert_eq!(render("![p](data:image/png;base64,AA)"), "<p>p</p>");
        assert_eq!(render("[x](\"onmouseover=alert(1))"), "<p><a href=\"&quot;onmouseover=alert(1)\">x</a></p>");
    }
}
