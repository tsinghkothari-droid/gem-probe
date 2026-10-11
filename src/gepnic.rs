use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

const USER_AGENT: &str = "gem-probe/0.1 (public by-date tender list; +https://github.com/tsinghkothari-droid/gem-probe)";

/// GePNIC hosts whose by-date page already prints tender rows.
/// The search form on the same page asks for a captcha. This reader never submits it.
const BOARDS: &[(&str, &str)] = &[
    (
        "Rajasthan",
        "https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
    (
        "Himachal Pradesh",
        "https://hptenders.gov.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
    (
        "Defence",
        "https://defproc.gov.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
    (
        "Haryana",
        "https://etenders.hry.nic.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
    (
        "Coal India",
        "https://coalindiatenders.nic.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
    (
        "Jammu and Kashmir",
        "https://jktenders.gov.in/nicgep/app?page=FrontEndListTendersbyDate&service=page",
    ),
];

#[derive(Debug, Clone)]
pub struct Notice {
    pub source: String,
    pub id: String,
    pub title: String,
    pub reference: String,
    pub organisation: String,
    pub closing: String,
    pub list_url: String,
}

impl Notice {
    pub fn text(&self) -> String {
        format!("{} {} {}", self.title, self.reference, self.organisation)
    }
}

pub async fn probe() -> Vec<(String, Result<usize, String>)> {
    let Ok(http) = client() else {
        return BOARDS
            .iter()
            .map(|(name, _)| ((*name).to_string(), Err("http client failed".into())))
            .collect();
    };
    let mut jobs = Vec::new();
    for (name, url) in BOARDS {
        let http = http.clone();
        let name = (*name).to_string();
        let url = (*url).to_string();
        jobs.push(tokio::spawn(async move {
            let result = fetch_one(&http, &name, &url).await.map(|rows| rows.len());
            (name, result)
        }));
    }
    let mut results = Vec::new();
    for job in jobs {
        if let Ok(item) = job.await {
            results.push(item);
        }
    }
    results
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn board_feeds() -> Vec<(String, Result<Vec<Notice>, String>)> {
    let http = match client() {
        Ok(http) => http,
        Err(err) => {
            return BOARDS
                .iter()
                .map(|(name, _)| ((*name).to_string(), Err(err.clone())))
                .collect();
        }
    };
    let mut jobs = Vec::new();
    for (name, url) in BOARDS {
        let http = http.clone();
        let name = (*name).to_string();
        let url = (*url).to_string();
        jobs.push(tokio::spawn(async move {
            let result = fetch_one(&http, &name, &url).await;
            (name, result)
        }));
    }
    let mut feeds = Vec::new();
    for job in jobs {
        if let Ok(item) = job.await {
            feeds.push(item);
        }
    }
    feeds
}

pub async fn fetch_open_boards() -> Vec<Notice> {
    let mut notices = Vec::new();
    for (name, result) in board_feeds().await {
        match result {
            Ok(rows) => {
                println!("{:4} rows  {name}", rows.len());
                notices.extend(rows);
            }
            Err(err) => eprintln!("skip {name}: {err}"),
        }
    }
    notices
}

async fn fetch_one(http: &reqwest::Client, name: &str, url: &str) -> Result<Vec<Notice>, String> {
    let html = http
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_board(name, url, &html))
}

pub fn parse_board(source: &str, list_url: &str, html: &str) -> Vec<Notice> {
    let mut notices = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in row_pattern().find_iter(html) {
        let row = row.as_str();
        let Some(id) = id_pattern().find(row).map(|m| m.as_str().to_string()) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let cells = cell_pattern()
            .find_iter(row)
            .map(|cell| strip_tags(cell.as_str()))
            .collect::<Vec<_>>();
        let title_cell = cells.get(4).cloned().unwrap_or_default();
        let (title, reference) = split_title(&title_cell, &id);
        notices.push(Notice {
            source: source.to_string(),
            id,
            title,
            reference,
            organisation: cells.get(5).cloned().unwrap_or_default(),
            closing: cells.get(2).cloned().unwrap_or_default(),
            list_url: list_url.to_string(),
        });
    }
    notices
}

fn split_title(cell: &str, id: &str) -> (String, String) {
    let parts = bracket_pattern()
        .captures_iter(cell)
        .filter_map(|caps| caps.get(1).map(|part| part.as_str().trim().to_string()))
        .filter(|part| part != id)
        .collect::<Vec<_>>();
    let title = parts.first().cloned().unwrap_or_else(|| cell.trim().to_string());
    let reference = parts.into_iter().skip(1).collect::<Vec<_>>().join(" ");
    (title, reference)
}

fn bracket_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"\[([^\[\]]+)\]").expect("bracket"))
}

fn strip_tags(html: &str) -> String {
    let text = tag_pattern().replace_all(html, " ");
    unescape(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn row_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r#"(?s)<tr class="(?:odd|even)".*?</tr>"#).expect("row"))
}

fn cell_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?s)<td\b[^>]*>.*?</td>").expect("cell"))
}

fn id_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"20\d{2}_[A-Z][A-Z0-9]*_\d+(?:_\d+)*").expect("id"))
}

fn tag_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?s)<[^>]+>").expect("tag"))
}

#[cfg(test)]
mod tests {
    use super::parse_board;

    const ROW: &str = r#"
<tr class="odd" id="informal_10">
<td align="center">4.</td>
<td align="center">02-Oct-2026 09:30 AM</td>
<td align="center">11-Oct-2026 06:00 PM</td>
<td align="center">12-Oct-2026 11:00 AM</td>
<td align="center"><a href="/nicgep/app?x=1">[Maintenance and repair work of roads]</a>
[EE PWD DIV. NAINWA NITNO-14/2026-27][2026_CEPWD_602010_10]
</td>
<td align="center">PWD - CE AND AS||ACE - KOTA||EE-NAINWA</td>
</tr>
"#;

    #[test]
    fn reads_one_by_date_row() {
        let notices = parse_board("Rajasthan", "https://example.test/list", ROW);
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].id, "2026_CEPWD_602010_10");
        assert_eq!(notices[0].title, "Maintenance and repair work of roads");
        assert_eq!(notices[0].organisation, "PWD - CE AND AS||ACE - KOTA||EE-NAINWA");
        assert_eq!(notices[0].closing, "11-Oct-2026 06:00 PM");
        assert!(notices[0].reference.contains("NAINWA"));
    }
}
