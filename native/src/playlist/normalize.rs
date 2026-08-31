//! Playlist normalization pipeline (import-time, pure functions).
//!
//! Chinese IPTV lists (iptv-org, domestic aggregators) share predictable
//! messiness: quality tags in names (`(1080p)`), geo/no-24/7 flags, English
//! station names (`Hunan TV`), traditional characters, content-type groups
//! instead of broadcaster groups, and useless dictionary ordering.
//!
//! The pipeline has three stages, all pure and independently testable:
//!   1. `clean_name`      — display-name cleanup (tags, variants, callsigns)
//!   2. `canonical_group` — rule-based grouping (央视 / 卫视 / 港澳台 / passthrough)
//!   3. `sort_channels`   — canonical group order + CCTV channel numbers
//!
//! Extending to other regions: add keyword/alias tables next to the CN ones
//! below (see `CN_*` consts) and extend `canonical_group`/`sort_key` — no
//! architectural change is needed.

use crate::playlist::model::{Channel, Playlist};

/// Groups produced by the CN rules, in canonical display order.
pub const GROUP_CCTV: &str = "央视";
pub const GROUP_PROVINCIAL: &str = "卫视";
pub const GROUP_GAT: &str = "港澳台";
/// Replacement for the zero-information groups some lists emit.
pub const GROUP_OTHER: &str = "其他";



/// Confident English → Chinese satellite-brand aliases observed in real
/// playlists. Applied after tag stripping, exact match on the cleaned name.
const CN_ENGLISH_ALIASES: &[(&str, &str)] = &[
    ("hunan tv", "湖南卫视"),
    ("dragon tv", "东方卫视"),
    ("dragon tv international", "东方卫视"),
    ("guangdong satellite tv", "广东卫视"),
    ("shenzhen satellite tv", "深圳卫视"),
    ("anhui tv", "安徽卫视"),
    ("hebei tv", "河北卫视"),
    ("zhejiang tv", "浙江卫视"),
    ("zhejiang tv international", "浙江卫视"),
    ("nei monggol tv", "内蒙古卫视"),
    ("nei mongol tv", "内蒙古卫视"),
];

/// Station keywords that mark a channel as Hong Kong / Macau / Taiwan.
/// Latin keywords are matched against the upper-cased name, CJK keywords
/// against the simplified name.
const CN_GAT_KEYWORDS_LATIN: &[&str] = &[
    "RTHK", "TVB", "HOY TV", "VIUTV", "TDM", "CANAL MACAU", "TVBS",
    "MOMO", "PTS", "FTV", "TTV", "CTV", "TAIWANPLUS",
];
const CN_GAT_KEYWORDS_CJK: &[&str] = &[
    "公视", "民视", "中视", "台视", "华视", "三立", "东森", "中天",
    "八大", "纬来", "大爱", "靖天", "龙祥", "霹雳", "博斯", "无线",
    "香港", "凤凰", "澳视", "澳广视",
];

/// Zero-information group titles replaced with 「其他」.
const GENERIC_GROUPS: &[&str] = &["undefined", "general", "other", "others", "misc"];

pub fn normalize_playlist(playlist: Playlist, smart_grouping: bool) -> Playlist {
    if !smart_grouping {
        return playlist;
    }

    let mut channels = playlist.channels;
    for channel in &mut channels {
        let cleaned = clean_name(&channel.name);
        channel.name = cleaned.clone();
        channel.group = canonical_group(&cleaned, &channel.group);
    }
    sort_channels(&mut channels);

    Playlist { channels, ..playlist }
}

/// Stage 1: clean a raw display name.
///
/// Strips quality tags `(1080p)`/`(576i)`, availability flags
/// `[Geo-blocked]`/`[Not 24/7]`, circled subscripts ⓈⒼⓎ, converts
/// traditional characters, drops redundant Latin callsigns (`BRTV 北京卫视`)
/// and maps known English station names to their Chinese brands.
pub fn clean_name(raw: &str) -> String {
    let simplified: String = raw.chars().map(simplified_char).collect();
    let stripped = strip_tags(&simplified);
    let decalled = strip_callsign_prefix(stripped.trim());
    let canonical = canonicalize_cctv_style(decalled.trim());

    let candidate = canonical.trim();
    if candidate.is_empty() {
        return raw.trim().to_string();
    }

    let lower = candidate.to_lowercase();
    for (alias, canonical) in CN_ENGLISH_ALIASES {
        if *alias == lower {
            return (*canonical).to_string();
        }
    }
    candidate.to_string()
}

/// Stage 2: derive the canonical group for a cleaned name.
///
/// Priority: 央视 > 卫视 > 港澳台 > keep original group title. Unknown or
/// generic original titles fall back to 「其他」.
pub fn canonical_group(clean_name: &str, original_group: &str) -> String {
    if is_cctv_family(clean_name) {
        return GROUP_CCTV.to_string();
    }
    if clean_name.ends_with("卫视") || is_known_satellite(clean_name) {
        return GROUP_PROVINCIAL.to_string();
    }
    if is_gat(clean_name) {
        return GROUP_GAT.to_string();
    }

    let trimmed = original_group.trim();
    if trimmed.is_empty() {
        return GROUP_OTHER.to_string();
    }
    if GENERIC_GROUPS.contains(&trimmed.to_lowercase().as_str()) {
        return GROUP_OTHER.to_string();
    }
    trimmed.to_string()
}

/// Stage 3: canonical ordering.
///
/// Group order: 央视, 卫视, 港澳台, then remaining groups alphabetically.
/// Inside 央视, channels sort by CCTV channel number (CCTV-5+ right after
/// CCTV-5, 4K/8K after CCTV-17, CGTN and pay-TV after the numbered block,
/// CETV last). Other groups keep their original relative order.
pub fn sort_channels(channels: &mut [Channel]) {
    let mut indexed: Vec<(usize, &mut Channel)> =
        channels.iter_mut().enumerate().collect();
    indexed.sort_by(|(left_idx, left), (right_idx, right)| {
        let left_key = channel_sort_key(left);
        let right_key = channel_sort_key(right);
        left_key
            .cmp(&right_key)
            .then_with(|| left_idx.cmp(right_idx))
    });
    let reordered: Vec<Channel> = indexed
        .into_iter()
        .map(|(_, channel)| channel.clone())
        .collect();
    channels.clone_from_slice(&reordered);
}

/// `(group_rank, group_name, in_group_class, number_x100, name)`
type ChannelSortKey = (u32, String, u32, u64, String);

fn channel_sort_key(channel: &Channel) -> ChannelSortKey {
    let rank = group_rank(&channel.group);
    match rank {
        0 => {
            let (class, num) = cctv_number(&channel.name);
            (0, channel.group.clone(), class, num, channel.name.clone())
        }
        1 | 2 => (rank, channel.group.clone(), 0, 0, channel.name.clone()),
        _ => (3, channel.group.clone(), 0, 0, String::new()),
    }
}

fn group_rank(group: &str) -> u32 {
    match group {
        GROUP_CCTV => 0,
        GROUP_PROVINCIAL => 1,
        GROUP_GAT => 2,
        _ => 3,
    }
}

/// In-group sort class within 央视: 0 = numbered CCTV, 1 = unnumbered
/// CCTV/CGTN (风云剧场 etc.), 2 = CETV.
fn cctv_number(name: &str) -> (u32, u64) {
    let upper = name.to_uppercase();

    if let Some(rest) = upper.strip_prefix("CETV") {
        let n = leading_number(rest).unwrap_or(0);
        return (2, n * 100);
    }

    let cctv_rest = upper.strip_prefix("CCTV");
    let cgtn_rest = upper.strip_prefix("CGTN");
    let rest = cctv_rest.or(cgtn_rest);

    let Some(rest) = rest else {
        return (1, 0)
    };

    // 4K/8K must be checked before the numeric parse, otherwise "CCTV-4K"
    // would sort as channel 4.
    if rest.contains("8K") {
        return (0, 19 * 100);
    }
    if rest.contains("4K") {
        return (0, 18 * 100);
    }

    if let Some(num) = leading_number(rest) {
        // CCTV-5+ sorts between CCTV-5 and CCTV-6.
        let bump = if rest.trim_start_matches(['-', ' ']).contains('+') {
            50
        } else {
            0
        };
        return (0, num * 100 + bump);
    }
    (1, 0)
}

fn leading_number(input: &str) -> Option<u64> {
    let digits: String = input
        .trim_start_matches(['-', ' ', '＋', '+'])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn is_cctv_family(name: &str) -> bool {
    let upper = name.to_uppercase();
    upper.contains("CCTV") || upper.contains("CGTN") || upper.contains("CETV")
        || name.starts_with("央视") || name.starts_with("中央")
}

/// Extra satellite brands whose cleaned name does not end with 卫视
/// (e.g. 「湖南卫视」 written as 「湖南台」). Keep small and confident.
fn is_known_satellite(name: &str) -> bool {
    name.ends_with("卫视台") || name.ends_with("卫星频道") || name == "湖南台"
}

fn is_gat(name: &str) -> bool {
    let upper = name.to_uppercase();
    CN_GAT_KEYWORDS_LATIN
        .iter()
        .any(|keyword| upper.contains(keyword))
        || CN_GAT_KEYWORDS_CJK.iter().any(|keyword| name.contains(keyword))
}

fn simplified_char(ch: char) -> char {
    // Traditional → simplified map for characters that actually appear in
    // channel names (北京衛視, 民視無線台, 徐州經濟生活, …). Extend freely;
    // unknown characters pass through untouched.
    match ch {
        '衛' | '衞' => '卫',
        '視' => '视',
        '電' => '电',
        '臺' => '台',
        '廣' => '广',
        '東' => '东',
        '國' => '国',
        '際' => '际',
        '經' => '经',
        '濟' => '济',
        '聞' => '闻',
        '綜' => '综',
        '蘇' => '苏',
        '龍' => '龙',
        '鄉' => '乡',
        '兒' => '儿',
        '樂' => '乐',
        '藝' => '艺',
        '體' => '体',
        '劇' => '剧',
        '財' => '财',
        '軍' => '军',
        '農' => '农',
        '記' => '记',
        '錄' => '录',
        '環' => '环',
        '聯' => '联',
        '網' => '网',
        '動' => '动',
        '畫' => '画',
        '賽' => '赛',
        '車' => '车',
        '買' => '买',
        '購' => '购',
        '門' => '门',
        '閩' => '闽',
        '粵' => '粤',
        '滬' => '沪',
        '魯' => '鲁',
        '陝' => '陕',
        '貴' => '贵',
        '雲' => '云',
        '寧' => '宁',
        '遼' => '辽',
        '無' => '无',
        '線' => '线',
        '頻' => '频',
        '紀' => '纪',
        '實' => '实',
        '戲' => '戏',
        '衆' => '众',
        '僑' => '侨',
        '風' => '风',
        '語' => '语',
        _ => ch,
    }
}

/// Remove `(...)`, `[...]` tags and circled marks that only carry metadata
/// (quality / geo / 24-7 flags), keeping meaningful brackets such as
/// `CCTV-4 中文国际（亚）` intact.
fn strip_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        if ch == '(' || ch == '[' {
            if let Some(end) = find_closing(&chars, index) {
                let inner: String = chars[index + 1..end].iter().collect();
                if is_metadata_tag(&inner) {
                    index = end + 1;
                    continue;
                }
            }
        }
        if is_circled_mark(ch) {
            index += 1;
            continue;
        }
        output.push(ch);
        index += 1;
    }

    output
}

fn find_closing(chars: &[char], open: usize) -> Option<usize> {
    let close = if chars[open] == '(' { ')' } else { ']' };
    chars[open + 1..].iter().position(|&c| c == close).map(|p| p + open + 1)
}

fn is_metadata_tag(inner: &str) -> bool {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return true;
    }
    // Quality: 720p, 1080i, 2160p, 406p, 576i, 60fps, 50fps …
    let body = trimmed
        .strip_suffix('p')
        .or_else(|| trimmed.strip_suffix('i'))
        .unwrap_or(trimmed);
    let digits: String = body.chars().filter(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() == body.len() {
        return true;
    }
    matches!(
        trimmed,
        "Geo-blocked" | "Not 24/7" | "Geo-blocked Ⓢ" | "Offline"
    )
}

fn is_circled_mark(ch: char) -> bool {
    // Ⓐ..⓿ range: enclosed alphanumerics used as subscription marks by lists.
    matches!(ch, '\u{24B6}'..='\u{24FF}')
}

/// Drop a redundant Latin callsign prefix when the rest of the name starts
/// with Chinese (`BRTV 北京卫视` → `北京卫视`); the "starts with" check keeps
/// real station names like `PTS Taigi` intact. Tokens with digits or hyphens
/// (`FZTV-1`, `CCTV-5`) are channel numbers, not callsigns, and are kept.
fn strip_callsign_prefix(input: &str) -> String {
    let Some((first, rest)) = input.split_once(' ') else {
        return input.to_string();
    };
    let looks_like_callsign = (2..=6).contains(&first.chars().count())
        && first.chars().all(|c| c.is_ascii_uppercase());
    let rest_starts_with_cjk = rest.chars().next().is_some_and(is_cjk);
    if looks_like_callsign && rest_starts_with_cjk {
        rest.to_string()
    } else {
        input.to_string()
    }
}

/// Unify channel-number spellings so the same station merges into one
/// multi-line card: `CCTV5` / `cctv5` / `CCTV 5` → `CCTV-5`, `CCTV5+` →
/// `CCTV-5+`. Suffixes after the number are preserved (`CCTV1 综合` →
/// `CCTV-1 综合`). Names without a leading number (`CCTV-风云剧场`) pass
/// through untouched.
fn canonicalize_cctv_style(name: &str) -> String {
    for prefix in ["CCTV", "CETV"] {
        if let Some(rest) = strip_prefix_ci(name, prefix) {
            let trimmed = rest.trim_start_matches(['-', ' ']);
            let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                continue;
            }
            let after = &trimmed[digits.len()..];
            let plus = after.strip_prefix('+').unwrap_or("");
            let tail = &after[plus.len()..];
            return format!("{prefix}-{digits}{plus}{tail}");
        }
    }
    name.to_string()
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    // `get` (unlike `split_at`) returns None when the byte index would land
    // inside a multi-byte character.
    let head = input.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(ch, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(name: &str, group: &str) -> Channel {
        Channel {
            id: name.to_string(),
            name: name.to_string(),
            stream_url: format!("https://example.com/{name}.m3u8"),
            group: group.to_string(),
            logo: None,
            tvg_id: None,
            user_agent: None,
            referrer: None,
        }
    }

    fn clean(raw: &str) -> String {
        clean_name(raw)
    }

    #[test]
    fn canonicalizes_cctv_number_variants() {
        assert_eq!(clean("CCTV5"), "CCTV-5");
        assert_eq!(clean("CCTV5+"), "CCTV-5+");
        assert_eq!(clean("cctv12"), "CCTV-12");
        assert_eq!(clean("CCTV 1 综合"), "CCTV-1 综合");
        assert_eq!(clean("CCTV-1"), "CCTV-1");
        assert_eq!(clean("CCTV-5+"), "CCTV-5+");
        assert_eq!(clean("CETV1"), "CETV-1");
        assert_eq!(clean("CCTV4K"), "CCTV-4K");
        // Unnumbered names stay untouched.
        assert_eq!(clean("CCTV-风云剧场"), "CCTV-风云剧场");
        assert_eq!(clean("CCTV+ 1"), "CCTV+ 1");
        assert_eq!(clean("CCTV-Billiards"), "CCTV-Billiards");
    }

    #[test]
    fn strips_quality_and_flag_tags() {
        assert_eq!(clean("CCTV-1 (1080p)"), "CCTV-1");
        assert_eq!(clean("CCTV+ 1 (600p) [Not 24/7]"), "CCTV+ 1");
        assert_eq!(clean("北京衛視 [Geo-blocked]"), "北京卫视");
        assert_eq!(clean("湖南卫视 (2160p)"), "湖南卫视");
        assert_eq!(clean("CCTV-4 中文国际（亚）"), "CCTV-4 中文国际（亚）");
        assert_eq!(clean("PTS Taigi (公視台語台) Ⓨ Ⓖ"), "PTS Taigi (公视台语台)");
    }

    #[test]
    fn strips_circled_marks_only_when_tag_is_metadata() {
        // (公視台語台) carries real information → kept, only ⓎⒼ removed.
        assert!(clean_name("PTS Taigi (公視台語台) Ⓨ Ⓖ").contains("公视台语台"));
        // (720p) is metadata → whole tag removed.
        assert!(!clean_name("ABP News (720p)").contains("720p"));
    }

    #[test]
    fn simplifies_traditional_names() {
        assert_eq!(clean("民視無線台"), "民视无线台");
        assert_eq!(clean("徐州經濟生活"), "徐州经济生活");
        assert_eq!(clean("黑龍江"), "黑龙江");
    }

    #[test]
    fn drops_latin_callsign_before_chinese() {
        assert_eq!(clean("BRTV 北京卫视"), "北京卫视");
        // FZTV-1 is a channel number, not a callsign → kept.
        assert_eq!(clean("FZTV-1 News 新闻综合频道"), "FZTV-1 News 新闻综合频道");
        // No CJK after the callsign → nothing dropped.
        assert_eq!(clean("TV BRICS Chinese"), "TV BRICS Chinese");
    }

    #[test]
    fn maps_english_satellite_brands() {
        assert_eq!(clean("Hunan TV (2160p)"), "湖南卫视");
        assert_eq!(clean("Dragon TV International (480p)"), "东方卫视");
        assert_eq!(clean("Shenzhen Satellite TV (2160p)"), "深圳卫视");
    }

    #[test]
    fn assigns_canonical_groups() {
        assert_eq!(canonical_group("CCTV-1", "Undefined"), GROUP_CCTV);
        assert_eq!(canonical_group("CGTN Documentary", "News"), GROUP_CCTV);
        assert_eq!(canonical_group("CETV1", "Education"), GROUP_CCTV);
        assert_eq!(canonical_group("湖南卫视", "Undefined"), GROUP_PROVINCIAL);
        assert_eq!(canonical_group("RTHK TV 31", "Hong Kong"), GROUP_GAT);
        assert_eq!(canonical_group("民视无线台", "Undefined"), GROUP_GAT);
        // Unknown but meaningful original groups pass through.
        assert_eq!(canonical_group("未知频道", "我的分组"), "我的分组");
        // Generic / empty originals collapse to 其他.
        assert_eq!(canonical_group("未知频道", "Undefined"), GROUP_OTHER);
        assert_eq!(canonical_group("未知频道", "General"), GROUP_OTHER);
        assert_eq!(canonical_group("未知频道", ""), GROUP_OTHER);
    }

    #[test]
    fn sorts_cctv_by_channel_number() {
        let mut channels = vec![
            channel("CCTV-13", GROUP_CCTV),
            channel("CCTV-1", GROUP_CCTV),
            channel("CCTV-17", GROUP_CCTV),
            channel("CCTV-5+", GROUP_CCTV),
            channel("CCTV-5", GROUP_CCTV),
            channel("CCTV-4K", GROUP_CCTV),
            channel("CCTV-8K", GROUP_CCTV),
            channel("CCTV-风云剧场", GROUP_CCTV),
            channel("CGTN", GROUP_CCTV),
            channel("CETV1", GROUP_CCTV),
            channel("CETV2", GROUP_CCTV),
        ];
        sort_channels(&mut channels);
        let names: Vec<&str> = channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "CCTV-1",
                "CCTV-5",
                "CCTV-5+",
                "CCTV-13",
                "CCTV-17",
                "CCTV-4K",
                "CCTV-8K",
                "CCTV-风云剧场",
                "CGTN",
                "CETV1",
                "CETV2",
            ]
        );
    }

    #[test]
    fn orders_canonical_groups_first() {
        let mut channels = vec![
            channel("浙江卫视", GROUP_PROVINCIAL),
            channel("其他频道", GROUP_OTHER),
            channel("CCTV-1", GROUP_CCTV),
            channel("RTHK TV 31", GROUP_GAT),
            channel("我的频道", "我的分组"),
        ];
        sort_channels(&mut channels);
        let groups: Vec<&str> = channels.iter().map(|c| c.group.as_str()).collect();
        assert_eq!(groups, vec![GROUP_CCTV, GROUP_PROVINCIAL, GROUP_GAT, GROUP_OTHER, "我的分组"]);
    }

    #[test]
    fn keeps_passthrough_group_order_stable() {
        let mut channels = vec![
            channel("频道B", "自定义分组"),
            channel("频道A", "自定义分组"),
            channel("频道C", "另一个分组"),
        ];
        sort_channels(&mut channels);
        let names: Vec<&str> = channels.iter().map(|c| c.name.as_str()).collect();
        // Rank-3 groups sort by group name; original order inside is kept.
        assert_eq!(names, vec!["频道C", "频道B", "频道A"]);
    }

    #[test]
    fn normalize_playlist_end_to_end() {
        let playlist = Playlist {
            channels: vec![
                channel("CCTV-13 新闻 (1080p)", "News"),
                channel("Hunan TV (2160p)", "Undefined"),
                channel("北京衛視 [Geo-blocked]", "Undefined"),
                channel("未知频道", "Undefined"),
            ],
            imported_at: "0".to_string(),
        };
        let result = normalize_playlist(playlist, true);
        let summary: Vec<(&str, &str)> = result
            .channels
            .iter()
            .map(|c| (c.name.as_str(), c.group.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("CCTV-13 新闻", GROUP_CCTV),
                ("北京卫视", GROUP_PROVINCIAL),
                ("湖南卫视", GROUP_PROVINCIAL),
                ("未知频道", GROUP_OTHER),
            ]
        );
    }

    #[test]
    fn normalize_playlist_passthrough_when_disabled() {
        let playlist = Playlist {
            channels: vec![channel("CCTV-13 新闻 (1080p)", "News")],
            imported_at: "0".to_string(),
        };
        let result = normalize_playlist(playlist, false);
        assert_eq!(result.channels[0].name, "CCTV-13 新闻 (1080p)");
        assert_eq!(result.channels[0].group, "News");
    }
}
