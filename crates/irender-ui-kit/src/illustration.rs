//! `ui-illustration`: drawings for error pages and empty states (not found, server error,
//! no data, out of stock, …), all original, flat, and coloured by the theme: the primary
//! colour, a secondary one (`color.brand-2`, else `color.chart-2`), and neutral tones
//! mixed from the text and background colours, so they follow a palette and dark mode.
//!
//! Each shape names its colour with `data-f` (fill) or `data-k` (stroke): `p` and `s`
//! for the primary and secondary colours, `pl` and `sl` for light tints of them, `n1`
//! to `n3` for neutrals from light to dark, and `w` for the background.

use crate::TEXT;
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use std::sync::LazyLock;

/// The drawings, by name: the inside of a 240 × 180 `<svg>`.
pub(crate) const ILLUSTRATIONS: &[(&str, &str)] = &[
    (
        "not-found",
        r#"<ellipse cx="120" cy="160" rx="92" ry="8" data-f="n1"/>
<circle cx="34" cy="34" r="3" data-f="s"/><circle cx="212" cy="28" r="2.5" data-f="p"/><circle cx="196" cy="140" r="2" data-f="s"/><circle cx="46" cy="132" r="2" data-f="pl"/>
<path d="M70 48 L38 104 H90 M72 48 V138" data-k="n3" stroke-width="14" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
<path d="M190 48 L158 104 H210 M192 48 V138" data-k="n3" stroke-width="14" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
<path d="M76 104 C76 84 164 66 164 86" data-k="sl" stroke-width="6" fill="none" stroke-linecap="round"/>
<circle cx="120" cy="92" r="32" data-f="p"/>
<circle cx="109" cy="80" r="7" data-f="pl"/><circle cx="132" cy="104" r="5" data-f="pl"/><circle cx="128" cy="78" r="3" data-f="pl"/>
<path d="M76 104 C76 116 164 104 164 86" data-k="s" stroke-width="6" fill="none" stroke-linecap="round"/>"#,
    ),
    (
        "server-error",
        r#"<ellipse cx="120" cy="160" rx="80" ry="8" data-f="n1"/>
<rect x="66" y="40" width="108" height="116" rx="10" data-f="n2"/>
<rect x="78" y="52" width="84" height="26" rx="5" data-f="w"/><rect x="78" y="85" width="84" height="26" rx="5" data-f="w"/><rect x="78" y="118" width="84" height="26" rx="5" data-f="w"/>
<rect x="88" y="62" width="34" height="6" rx="3" data-f="n1"/><rect x="88" y="95" width="34" height="6" rx="3" data-f="n1"/><rect x="88" y="128" width="34" height="6" rx="3" data-f="n1"/>
<circle cx="148" cy="65" r="5" data-f="p"/><circle cx="148" cy="98" r="5" data-f="s"/><circle cx="148" cy="131" r="5" data-f="p"/>
<path d="M176 14 L150 62 H168 L154 100 L196 46 H176 L190 14 Z" data-f="s"/>
<circle cx="50" cy="40" r="10" data-f="n1"/><circle cx="38" cy="58" r="6" data-f="n1"/><circle cx="58" cy="24" r="5" data-f="n1"/>"#,
    ),
    (
        "forbidden",
        r#"<ellipse cx="120" cy="160" rx="74" ry="8" data-f="n1"/>
<path d="M120 18 L176 38 V82 C176 118 152 142 120 154 C88 142 64 118 64 82 V38 Z" data-f="sl"/>
<path d="M120 30 L166 46 V82 C166 112 146 132 120 142 Z" data-f="s" opacity=".35"/>
<path d="M100 84 V70 C100 58 108 50 120 50 C132 50 140 58 140 70 V84" data-k="n3" stroke-width="9" fill="none" stroke-linecap="round"/>
<rect x="88" y="80" width="64" height="50" rx="9" data-f="p"/>
<circle cx="120" cy="100" r="7" data-f="w"/><rect x="117" y="104" width="6" height="14" rx="3" data-f="w"/>"#,
    ),
    (
        "maintenance",
        r#"<ellipse cx="120" cy="160" rx="92" ry="8" data-f="n1"/>
<rect x="30" y="30" width="150" height="104" rx="10" data-f="n1"/>
<path d="M30 40 A10 10 0 0 1 40 30 H170 A10 10 0 0 1 180 40 V50 H30 Z" data-f="n2"/>
<circle cx="44" cy="40" r="3.5" data-f="w"/><circle cx="56" cy="40" r="3.5" data-f="w"/><circle cx="68" cy="40" r="3.5" data-f="w"/>
<rect x="46" y="66" width="70" height="8" rx="4" data-f="n2"/><rect x="46" y="82" width="52" height="8" rx="4" data-f="n2"/><rect x="46" y="98" width="62" height="8" rx="4" data-f="n2"/>
<g transform="translate(166 108)"><path d="M-6 -36 H6 L8 -27 L16 -23 L24 -29 L32 -21 L26 -13 L30 -5 L38 -4 V8 L29 10 L25 18 L31 26 L23 34 L15 28 L7 32 L6 40 H-6 L-8 32 L-16 28 L-24 34 L-32 26 L-26 18 L-30 10 L-38 8 V-4 L-30 -6 L-26 -14 L-32 -22 L-24 -30 L-16 -24 L-8 -28 Z" data-f="p"/><circle r="12" data-f="w"/></g>
<path d="M200 40 L150 90" data-k="s" stroke-width="10" stroke-linecap="round"/>
<path d="M196 30 A14 14 0 1 0 212 48 L202 46 L198 36 Z" data-f="s"/>"#,
    ),
    (
        "offline",
        r#"<ellipse cx="120" cy="160" rx="84" ry="8" data-f="n1"/>
<path d="M66 128 H176 A30 30 0 0 0 170 69 A40 40 0 0 0 94 62 A34 34 0 0 0 66 128 Z" data-f="n1"/>
<path d="M86 100 A48 48 0 0 1 154 100" data-k="pl" stroke-width="9" fill="none" stroke-linecap="round"/>
<path d="M98 112 A30 30 0 0 1 142 112" data-k="p" stroke-width="9" fill="none" stroke-linecap="round"/>
<circle cx="120" cy="126" r="7" data-f="p"/>
<path d="M78 58 L162 142" data-k="s" stroke-width="9" stroke-linecap="round"/>"#,
    ),
    (
        "no-data",
        r#"<ellipse cx="120" cy="160" rx="92" ry="8" data-f="n1"/>
<rect x="36" y="26" width="150" height="120" rx="10" data-f="w" data-k="n2" stroke-width="2"/>
<path d="M56 46 V126 H170" data-k="n3" stroke-width="3" fill="none" stroke-linecap="round"/>
<rect x="70" y="96" width="16" height="30" rx="3" data-k="n2" stroke-width="2" stroke-dasharray="4 4" fill="none"/>
<rect x="96" y="76" width="16" height="50" rx="3" data-k="n2" stroke-width="2" stroke-dasharray="4 4" fill="none"/>
<rect x="122" y="86" width="16" height="40" rx="3" data-k="n2" stroke-width="2" stroke-dasharray="4 4" fill="none"/>
<rect x="148" y="66" width="16" height="60" rx="3" data-k="n2" stroke-width="2" stroke-dasharray="4 4" fill="none"/>
<circle cx="176" cy="104" r="22" data-f="pl" data-k="p" stroke-width="8"/>
<path d="M192 120 L212 140" data-k="s" stroke-width="10" stroke-linecap="round"/>"#,
    ),
    (
        "no-results",
        r#"<ellipse cx="120" cy="160" rx="84" ry="8" data-f="n1"/>
<path d="M58 24 H134 L162 52 V146 H58 Z" data-f="w" data-k="n2" stroke-width="2" stroke-linejoin="round"/>
<path d="M134 24 V52 H162" data-f="n1" data-k="n2" stroke-width="2" stroke-linejoin="round"/>
<rect x="74" y="66" width="60" height="7" rx="3.5" data-f="n1"/><rect x="74" y="82" width="44" height="7" rx="3.5" data-f="n1"/><rect x="74" y="98" width="52" height="7" rx="3.5" data-f="n1"/>
<circle cx="146" cy="106" r="30" data-f="w" data-k="p" stroke-width="9"/>
<path d="M168 128 L196 156" data-k="p" stroke-width="12" stroke-linecap="round"/>
<path d="M136 98 A10 10 0 1 1 150 107 C147 109 146 111 146 115" data-k="s" stroke-width="6" fill="none" stroke-linecap="round"/>
<circle cx="146" cy="124" r="3.8" data-f="s"/>"#,
    ),
    (
        "out-of-stock",
        r#"<ellipse cx="120" cy="160" rx="88" ry="8" data-f="n1"/>
<path d="M60 70 L120 54 L180 70 L120 86 Z" data-f="n3" opacity=".6"/>
<path d="M60 70 V132 L120 150 V86 Z" data-f="sl"/>
<path d="M180 70 V132 L120 150 V86 Z" data-f="s" opacity=".75"/>
<path d="M60 70 L40 92 L100 110 L120 86 Z" data-f="sl" data-k="s" stroke-width="2" stroke-linejoin="round"/>
<path d="M180 70 L200 92 L140 110 L120 86 Z" data-f="s" stroke-linejoin="round"/>
<path d="M150 22 H196 L208 40 L196 58 H150 Z" data-f="p"/><circle cx="160" cy="40" r="5" data-f="w"/>
<path d="M176 32 L190 48 M190 32 L176 48" data-k="w" stroke-width="4" stroke-linecap="round"/>
<path d="M150 40 C138 40 132 48 132 60" data-k="n3" stroke-width="2" fill="none"/>"#,
    ),
    (
        "empty-cart",
        r#"<ellipse cx="120" cy="160" rx="88" ry="8" data-f="n1"/>
<path d="M30 38 H54 L74 118 H170 L190 62 H64" data-k="n3" stroke-width="8" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M72 62 H186 L170 110 H82 Z" data-f="n1"/>
<circle cx="86" cy="140" r="12" data-f="p"/><circle cx="160" cy="140" r="12" data-f="p"/><circle cx="86" cy="140" r="4" data-f="w"/><circle cx="160" cy="140" r="4" data-f="w"/>
<path d="M104 80 H150 M110 94 H144" data-k="sl" stroke-width="5" stroke-linecap="round" stroke-dasharray="2 10"/>
<path d="M200 30 L212 18 M206 44 L222 40 M194 22 L196 6" data-k="s" stroke-width="4" stroke-linecap="round"/>"#,
    ),
    (
        "empty-inbox",
        r#"<ellipse cx="120" cy="160" rx="88" ry="8" data-f="n1"/>
<path d="M50 96 L74 54 H166 L190 96 V140 A6 6 0 0 1 184 146 H56 A6 6 0 0 1 50 140 Z" data-f="pl"/>
<path d="M50 96 H90 L98 112 H142 L150 96 H190 V140 A6 6 0 0 1 184 146 H56 A6 6 0 0 1 50 140 Z" data-f="p"/>
<path d="M150 26 L206 12 L180 62 L168 46 L150 26 Z M168 46 L206 12" data-f="s" data-k="s" stroke-width="2" stroke-linejoin="round"/>
<path d="M168 46 L164 60 L176 52" data-f="sl"/>
<path d="M146 30 C122 40 106 30 92 40" data-k="n2" stroke-width="3" stroke-dasharray="5 6" fill="none" stroke-linecap="round"/>"#,
    ),
    (
        "no-notifications",
        r#"<ellipse cx="120" cy="160" rx="72" ry="8" data-f="n1"/>
<path d="M120 30 C94 30 78 50 78 76 V108 L66 124 H174 L162 108 V76 C162 50 146 30 120 30 Z" data-f="p"/>
<rect x="114" y="20" width="12" height="14" rx="6" data-f="p"/>
<path d="M104 126 A16 16 0 0 0 136 126 Z" data-f="s"/>
<path d="M92 72 A30 30 0 0 1 104 50" data-k="pl" stroke-width="6" fill="none" stroke-linecap="round"/>
<path d="M172 34 H190 L172 52 H190 M196 14 H208 L196 26 H208" data-k="s" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>"#,
    ),
    (
        "session-expired",
        r#"<ellipse cx="120" cy="160" rx="70" ry="8" data-f="n1"/>
<rect x="78" y="22" width="84" height="12" rx="6" data-f="n3"/><rect x="78" y="140" width="84" height="12" rx="6" data-f="n3"/>
<path d="M88 34 H152 V48 C152 66 132 78 124 87 C132 96 152 108 152 126 V140 H88 V126 C88 108 108 96 116 87 C108 78 88 66 88 48 Z" data-f="w" data-k="n2" stroke-width="3"/>
<path d="M98 50 H142 C142 62 128 70 120 78 C112 70 98 62 98 50 Z" data-f="p"/>
<path d="M120 92 V118" data-k="pl" stroke-width="3" stroke-dasharray="2 5" stroke-linecap="round"/>
<path d="M96 138 C100 124 110 118 120 118 C130 118 140 124 144 138 Z" data-f="s"/>
<circle cx="190" cy="52" r="20" data-f="sl"/><path d="M190 40 V52 L198 58" data-k="s" stroke-width="4" fill="none" stroke-linecap="round"/>"#,
    ),
    (
        "payment-failed",
        r#"<ellipse cx="120" cy="160" rx="88" ry="8" data-f="n1"/>
<rect x="34" y="40" width="150" height="96" rx="12" data-f="p"/>
<rect x="34" y="60" width="150" height="16" data-f="n3" opacity=".7"/>
<rect x="50" y="90" width="28" height="20" rx="4" data-f="sl"/>
<rect x="50" y="118" width="70" height="6" rx="3" data-f="pl"/>
<circle cx="178" cy="122" r="28" data-f="s"/>
<path d="M166 110 L190 134 M190 110 L166 134" data-k="w" stroke-width="7" stroke-linecap="round"/>"#,
    ),
    (
        "coming-soon",
        r#"<ellipse cx="120" cy="160" rx="60" ry="8" data-f="n1"/>
<circle cx="40" cy="40" r="3" data-f="s"/><circle cx="200" cy="56" r="3" data-f="p"/><circle cx="64" cy="100" r="2" data-f="pl"/><circle cx="186" cy="20" r="2" data-f="sl"/>
<path d="M120 12 C146 30 154 64 146 108 H94 C86 64 94 30 120 12 Z" data-f="p"/>
<circle cx="120" cy="58" r="13" data-f="w" data-k="s" stroke-width="5"/>
<path d="M94 84 L72 112 L94 108 Z M146 84 L168 112 L146 108 Z" data-f="s"/>
<path d="M102 112 H138 C138 130 128 144 120 156 C112 144 102 130 102 112 Z" data-f="sl"/>
<path d="M110 112 H130 C130 124 125 134 120 142 C115 134 110 124 110 112 Z" data-f="s"/>"#,
    ),
    (
        "all-done",
        r#"<ellipse cx="120" cy="160" rx="64" ry="8" data-f="n1"/>
<circle cx="120" cy="84" r="52" data-f="pl"/>
<circle cx="120" cy="84" r="40" data-f="p"/>
<path d="M100 86 L114 100 L142 70" data-k="w" stroke-width="10" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<rect x="44" y="34" width="10" height="5" rx="2.5" transform="rotate(-30 49 36)" data-f="s"/>
<rect x="186" y="46" width="12" height="5" rx="2.5" transform="rotate(25 192 48)" data-f="s"/>
<rect x="56" y="122" width="10" height="5" rx="2.5" transform="rotate(40 61 124)" data-f="sl"/>
<rect x="178" y="118" width="10" height="5" rx="2.5" transform="rotate(-20 183 120)" data-f="s"/>
<circle cx="74" cy="60" r="3.5" data-f="s"/><circle cx="170" cy="28" r="3" data-f="p"/><circle cx="196" cy="88" r="3" data-f="sl"/><circle cx="40" cy="92" r="3" data-f="p"/>"#,
    ),
    (
        "no-access",
        r#"<ellipse cx="120" cy="160" rx="80" ry="8" data-f="n1"/>
<path d="M62 20 H132 L158 46 V150 H62 Z" data-f="w" data-k="n2" stroke-width="2" stroke-linejoin="round"/>
<path d="M132 20 V46 H158" data-f="n1" data-k="n2" stroke-width="2" stroke-linejoin="round"/>
<rect x="78" y="60" width="56" height="7" rx="3.5" data-f="n1"/><rect x="78" y="76" width="42" height="7" rx="3.5" data-f="n1"/><rect x="78" y="92" width="50" height="7" rx="3.5" data-f="n1"/><rect x="78" y="108" width="30" height="7" rx="3.5" data-f="n1"/>
<path d="M156 112 V100 C156 90 162 84 170 84 C178 84 184 90 184 100 V112" data-k="n3" stroke-width="7" fill="none" stroke-linecap="round"/>
<rect x="146" y="106" width="48" height="38" rx="8" data-f="p"/>
<circle cx="170" cy="121" r="5.5" data-f="w"/><rect x="167.5" y="123" width="5" height="11" rx="2.5" data-f="w"/>
<circle cx="40" cy="40" r="3" data-f="s"/><circle cx="200" cy="40" r="2.5" data-f="sl"/><circle cx="40" cy="120" r="2" data-f="pl"/>"#,
    ),
    (
        "upload-failed",
        r#"<ellipse cx="120" cy="160" rx="84" ry="8" data-f="n1"/>
<path d="M62 118 H172 A30 30 0 0 0 166 59 A40 40 0 0 0 90 52 A34 34 0 0 0 62 118 Z" data-f="n1"/>
<path d="M120 140 V80" data-k="p" stroke-width="10" stroke-linecap="round"/>
<path d="M98 100 L120 78 L142 100" data-k="p" stroke-width="10" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M108 150 H132" data-k="pl" stroke-width="6" stroke-linecap="round"/>
<circle cx="176" cy="122" r="22" data-f="s"/>
<path d="M167 113 L185 131 M185 113 L167 131" data-k="w" stroke-width="6" stroke-linecap="round"/>
<circle cx="44" cy="34" r="3" data-f="s"/><circle cx="206" cy="30" r="2.5" data-f="p"/>"#,
    ),
    (
        "too-many-requests",
        r#"<ellipse cx="120" cy="160" rx="84" ry="8" data-f="n1"/>
<path d="M48 128 A72 72 0 0 1 84 65.6" data-k="pl" stroke-width="18" fill="none"/>
<path d="M84 65.6 A72 72 0 0 1 156 65.6" data-k="p" stroke-width="18" fill="none"/>
<path d="M156 65.6 A72 72 0 0 1 192 128" data-k="s" stroke-width="18" fill="none"/>
<path d="M66 128 A54 54 0 0 1 174 128" data-k="n1" stroke-width="4" stroke-dasharray="2 8" fill="none" stroke-linecap="round"/>
<path d="M120 128 L170 104" data-k="n3" stroke-width="6" stroke-linecap="round"/>
<circle cx="120" cy="128" r="11" data-f="n3"/><circle cx="120" cy="128" r="4" data-f="w"/>
<path d="M200 30 L212 18 M206 44 L222 40 M194 22 L196 6" data-k="s" stroke-width="4" stroke-linecap="round"/>
<path d="M18 74 H36 M14 90 H30" data-k="n2" stroke-width="4" stroke-linecap="round"/>"#,
    ),
    (
        "no-people",
        r#"<ellipse cx="120" cy="160" rx="88" ry="8" data-f="n1"/>
<rect x="34" y="34" width="112" height="44" rx="8" data-f="w" data-k="n2" stroke-width="2"/>
<circle cx="56" cy="56" r="12" data-f="pl"/><rect x="76" y="48" width="52" height="6" rx="3" data-f="n2"/><rect x="76" y="60" width="34" height="6" rx="3" data-f="n1"/>
<rect x="34" y="92" width="112" height="44" rx="8" fill="none" data-k="n2" stroke-width="2" stroke-dasharray="5 5"/>
<circle cx="56" cy="114" r="12" fill="none" data-k="n2" stroke-width="2" stroke-dasharray="4 4"/><rect x="76" y="106" width="52" height="6" rx="3" data-f="n1"/><rect x="76" y="118" width="34" height="6" rx="3" data-f="n1"/>
<circle cx="168" cy="96" r="28" data-f="w" data-k="p" stroke-width="9"/>
<path d="M189 117 L212 140" data-k="p" stroke-width="12" stroke-linecap="round"/>
<path d="M158 88 A10 10 0 1 1 172 97 C169 99 168 101 168 105" data-k="s" stroke-width="6" fill="none" stroke-linecap="round"/>
<circle cx="168" cy="114" r="3.8" data-f="s"/>"#,
    ),
    (
        "deleted",
        r#"<ellipse cx="120" cy="160" rx="72" ry="8" data-f="n1"/>
<path d="M78 66 H162 L154 148 A8 8 0 0 1 146 155 H94 A8 8 0 0 1 86 148 Z" data-f="p"/>
<path d="M104 82 V138 M120 82 V138 M136 82 V138" data-k="pl" stroke-width="6" stroke-linecap="round"/>
<g transform="rotate(-24 74 64)"><rect x="70" y="54" width="100" height="12" rx="6" data-f="s"/><path d="M104 54 V48 A4 4 0 0 1 108 44 H132 A4 4 0 0 1 136 48 V54" data-k="s" stroke-width="5" fill="none"/></g>
<g transform="rotate(16 186 30)"><rect x="170" y="10" width="32" height="40" rx="3" data-f="w" data-k="n2" stroke-width="2"/><path d="M177 22 H195 M177 30 H191 M177 38 H193" data-k="n1" stroke-width="3" stroke-linecap="round"/></g>
<path d="M180 58 C176 66 166 70 156 70" data-k="n2" stroke-width="3" stroke-dasharray="4 6" fill="none" stroke-linecap="round"/>"#,
    ),
    (
        "empty-calendar",
        r#"<ellipse cx="120" cy="160" rx="84" ry="8" data-f="n1"/>
<rect x="50" y="34" width="140" height="116" rx="10" data-f="w" data-k="n2" stroke-width="2"/>
<path d="M50 44 A10 10 0 0 1 60 34 H180 A10 10 0 0 1 190 44 V62 H50 Z" data-f="p"/>
<rect x="80" y="24" width="8" height="20" rx="4" data-f="n3"/><rect x="152" y="24" width="8" height="20" rx="4" data-f="n3"/>
<rect x="64" y="76" width="16" height="14" rx="3" data-f="n1"/><rect x="88" y="76" width="16" height="14" rx="3" data-f="n1"/><rect x="112" y="76" width="16" height="14" rx="3" data-f="n1"/><rect x="136" y="76" width="16" height="14" rx="3" data-f="n1"/><rect x="160" y="76" width="16" height="14" rx="3" data-f="n1"/><rect x="64" y="100" width="16" height="14" rx="3" data-f="n1"/><rect x="88" y="100" width="16" height="14" rx="3" data-f="n1"/><rect x="136" y="100" width="16" height="14" rx="3" data-f="n1"/><rect x="160" y="100" width="16" height="14" rx="3" data-f="n1"/><rect x="64" y="124" width="16" height="14" rx="3" data-f="n1"/><rect x="88" y="124" width="16" height="14" rx="3" data-f="n1"/><rect x="112" y="124" width="16" height="14" rx="3" data-f="n1"/><rect x="136" y="124" width="16" height="14" rx="3" data-f="n1"/><rect x="160" y="124" width="16" height="14" rx="3" data-f="n1"/>
<rect x="112" y="100" width="16" height="14" rx="3" fill="none" data-k="s" stroke-width="2.5" stroke-dasharray="3 3"/>
<path d="M196 30 H208 L196 42 H208 M210 12 H220 L210 22 H220" data-k="s" stroke-width="3.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>"#,
    ),
    (
        "welcome",
        r#"<ellipse cx="120" cy="160" rx="92" ry="8" data-f="n1"/>
<path d="M84 156 H156 L182 168 H58 Z" data-f="sl" opacity=".8"/>
<rect x="78" y="24" width="84" height="134" rx="4" data-f="n2"/>
<rect x="84" y="30" width="72" height="126" data-f="sl"/>
<path d="M84 30 L56 40 V150 L84 156 Z" data-f="p"/>
<circle cx="64" cy="98" r="4" data-f="w"/>
<rect x="96" y="150" width="48" height="8" rx="4" data-f="s"/>
<path d="M192 26 L196 38 L208 42 L196 46 L192 58 L188 46 L176 42 L188 38 Z" data-f="s"/>
<path d="M40 52 L42 59 L49 61 L42 63 L40 70 L38 63 L31 61 L38 59 Z" data-f="p"/>
<circle cx="208" cy="80" r="3" data-f="pl"/><circle cx="30" cy="100" r="2.5" data-f="s"/>"#,
    ),
];

/// The names, for the prop's enum and the docs.
pub(crate) const NAMES: &[&str] = &[
    "not-found", "server-error", "forbidden", "maintenance", "offline", "no-data", "no-results", "out-of-stock", "empty-cart",
    "empty-inbox", "no-notifications", "session-expired", "payment-failed", "coming-soon", "all-done", "no-access", "upload-failed",
    "too-many-requests", "no-people", "deleted", "empty-calendar", "welcome",
];

/// The colours, by role. The background is the surface the drawing sits on.
const BG: &str = "var(--ill-bg, var(--ui-bg, var(--color-background, var(--color-surface))))";

static STYLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mix = |c: &str, pct: u8| format!("color-mix(in srgb, {c} {pct}%, {BG})");
    let p = "var(--ill-p, var(--color-primary))";
    let s = "var(--ill-s, var(--color-brand-2, var(--color-chart-2, #f59e0b)))";
    let t = TEXT;
    let mut b = StyleBlock::from_pairs(&[("display", "block"), ("max-width", "100%"), ("height", "auto"), ("flex-shrink", "0")]);
    for (k, v) in [
        ("p", p.to_string()),
        ("s", s.to_string()),
        ("pl", mix(p, 22)),
        ("sl", mix(s, 30)),
        ("n1", mix(t, 7)),
        ("n2", mix(t, 18)),
        ("n3", mix(t, 55)),
        ("w", BG.to_string()),
    ] {
        b = b.nest(format!("& [data-f={k}]"), StyleBlock::new().set("fill", v.clone()));
        b = b.nest(format!("& [data-k={k}]"), StyleBlock::new().set("stroke", v));
    }
    b
});

pub struct Illustration;

const PROPS: &[PropDef] = &[
    PropDef::new("name", PropKind::Enum(NAMES), "The drawing").required(),
    PropDef::new("size", PropKind::Any, "Width (default 240); the height follows, at 3 : 4"),
    PropDef::new("primary", PropKind::Any, "Its main colour, a CSS colour or token (default `color.primary`)"),
    PropDef::new("secondary", PropKind::Any, "Its second colour (default `color.brand-2`, else `color.chart-2`)"),
    PropDef::new("label", PropKind::Any, "Describe it for screen readers; without one it is decoration"),
];

impl Component for Illustration {
    fn name(&self) -> &'static str {
        "ui-illustration"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A drawing for an error page or an empty state (not found, server error, no data, out of stock, …), coloured by the theme's primary and secondary colours.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let name = cx.prop_string(node, "name").unwrap_or_default();
        let Some((_, body)) = ILLUSTRATIONS.iter().find(|(n, _)| *n == name) else {
            return Err(RenderError::new(node, format!("no illustration `{name}`; use one of {}", NAMES.join(", "))));
        };
        let mut colours = StyleBlock::new();
        for (prop, var) in [("primary", "--ill-p"), ("secondary", "--ill-s")] {
            if let Some(v) = node.prop(prop)
                && let Some(c) = cx.style_value(node, var, v)?
            {
                colours.push(var, c);
            }
        }
        if let Some(v) = node.prop("size")
            && let Some(w) = cx.style_value(node, "width", v)?
        {
            colours.push("width", w);
        }
        let mut el = cx.element("svg", node);
        el.base_style_ref(&STYLE).base_style(colours).attr("viewBox", "0 0 240 180").attr("width", "240").attr("height", "180");
        match node.prop("label") {
            Some(l) => {
                el.attr("role", "img").attr("aria-label", l.clone());
            }
            None => {
                el.attr("aria-hidden", "true").attr("focusable", "false");
            }
        }
        cx.open(&el, node)?;
        cx.markup(body);
        cx.close("svg");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_has_a_drawing_with_only_known_colours() {
        assert_eq!(NAMES.len(), ILLUSTRATIONS.len());
        for (name, body) in ILLUSTRATIONS {
            assert!(NAMES.contains(name), "{name}");
            for attr in ["data-f=\"", "data-k=\""] {
                for part in body.split(attr).skip(1) {
                    let role = &part[..part.find('"').unwrap()];
                    assert!(["p", "s", "pl", "sl", "n1", "n2", "n3", "w"].contains(&role), "{name}: {attr}{role}");
                }
            }
            assert!(!body.contains("fill=\"#") && !body.contains("stroke=\"#"), "{name} has a fixed colour");
        }
    }
}
