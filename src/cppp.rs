use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

const USER_AGENT: &str = "gem-probe/0.1 (public CPPP tender list; +https://github.com/tsinghkothari-droid/gem-probe)";

/// Public lists that already print tender titles. The keyword box on the same site asks for an image code.
const FEEDS: &[(&str, &str, u32)] = &[
    (
        "CPPP states",
        "https://eprocure.gov.in/cppp/latestactivetendersnew/mmpdata",
        2,
    ),
    (
        "CPPP central",
        "https://eprocure.gov.in/cppp/latestactivetendersnew/cpppdata",
        2,
    ),
    (
        "CPPP high value",
        "https://eprocure.gov.in/cppp/highvaluetenders/cpppdata",
        2,
    ),
    (
        "CPPP closing today",
        "https://eprocure.gov.in/cppp/tendersclosingbydays/bytoday",
        1,
    ),
];

#[derive(Debug, Clone)]
pub struct Notice {
    pub source: String,
    pub id: String,
    pub title: String,
    pub buyer: String,
    pub published: String,
    pub closing: String,
    pub link: String,
}

impl Notice {
    pub fn text(&self) -> String {
        format!("{} {}", self.title, self.buyer)
    }
}

pub async fn probe() -> Vec<(String, Result<usize, String>)> {
    let Ok(http) = client() else {
        return FEEDS
            .iter()
            .map(|(name, _, _)| ((*name).to_string(), Err("http client failed".into())))
            .collect();
    };
    let mut jobs = Vec::new();
    for (name, url, _) in FEEDS {
        let http = http.clone();
        let name = (*name).to_string();
        let url = (*url).to_string();
        jobs.push(tokio::spawn(async move {
            let result = fetch_feed(&http, &name, &url, 1).await.map(|rows| rows.len());
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
        .timeout(Duration::from_secs(40))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn fetch_list(name: &str, url: &str, pages: u32) -> Vec<Notice> {
    let http = match reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(40))
        .build()
    {
        Ok(http) => http,
        Err(err) => {
            eprintln!("skip {name}: {err}");
            return Vec::new();
        }
    };
    fetch_feed(&http, name, url, pages).await.unwrap_or_else(|err| {
        eprintln!("skip {name}: {err}");
        Vec::new()
    })
}

pub async fn list_feeds() -> Vec<(String, Result<Vec<Notice>, String>)> {
    let http = match client() {
        Ok(http) => http,
        Err(err) => {
            return FEEDS
                .iter()
                .map(|(name, _, _)| ((*name).to_string(), Err(err.clone())))
                .collect();
        }
    };
    let mut jobs = Vec::new();
    for (name, url, pages) in FEEDS {
        let http = http.clone();
        let name = (*name).to_string();
        let url = (*url).to_string();
        let pages = *pages;
        jobs.push(tokio::spawn(async move {
            let result = fetch_feed(&http, &name, &url, pages).await;
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

async fn fetch_feed(http: &reqwest::Client, name: &str, url: &str, pages: u32) -> Result<Vec<Notice>, String> {
    let mut notices = Vec::new();
    for page in 1..=pages {
        let page_url = if page == 1 { url.to_string() } else { format!("{url}?page={page}") };
        let html = http
            .get(&page_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let rows = parse_list(name, &html);
        if rows.is_empty() && page > 1 {
            break;
        }
        notices.extend(rows);
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Ok(notices)
}

pub fn parse_list(source: &str, html: &str) -> Vec<Notice> {
    let mut notices = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in html.split("<tr").skip(1) {
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
        if cells.len() < 6 {
            continue;
        }
        let title = anchor_pattern()
            .captures(row)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_tags(m.as_str()))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| title_before_id(&cells[4], &id));
        let link = href_pattern()
            .captures(row)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        notices.push(Notice {
            source: source.to_string(),
            id,
            title,
            buyer: cells[5].clone(),
            published: cells.get(1).cloned().unwrap_or_default(),
            closing: cells.get(2).cloned().unwrap_or_default(),
            link,
        });
    }
    notices
}

fn title_before_id(cell: &str, id: &str) -> String {
    cell.split(id)
        .next()
        .unwrap_or(cell)
        .trim()
        .trim_end_matches('/')
        .trim()
        .to_string()
}

fn strip_tags(html: &str) -> String {
    unescape(&tag_pattern().replace_all(html, " "))
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

fn id_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"20\d{2}_[A-Z][A-Z0-9]*_\d+(?:_\d+)*").expect("id"))
}

fn cell_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?s)<td\b[^>]*>.*?</td>").expect("cell"))
}

fn anchor_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?s)<a\b[^>]*>(.*?)</a>").expect("anchor"))
}

fn href_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r#"href="(https://eprocure\.gov\.in/cppp/tendersfullview[^"]+)""#).expect("href"))
}

#[cfg(test)]
mod coverage {
    use super::fetch_list;
    use crate::profile::{match_bid, Profile};

    #[tokio::test]
    #[ignore]
    async fn coverage() {
        let profile: Profile = serde_json::from_str(include_str!("../profiles/navoday-sciences.json")).unwrap();
        let lists = [
            ("CPPP states", "https://eprocure.gov.in/cppp/latestactivetendersnew/mmpdata"),
            ("CPPP central", "https://eprocure.gov.in/cppp/latestactivetendersnew/cpppdata"),
        ];
        let watch = ["Gujarat", "Karnataka", "Maharashtra", "Uttar Pradesh", "West Bengal", "Rajasthan"];
        let mut state_link = None;
        for (name, url) in lists {
            let rows = fetch_list(name, url, 5).await;
            assert!(!rows.is_empty(), "{name} returned no titles");
            let mut hits: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
            let mut kept = 0u32;
            for row in &rows {
                if let Some(term) = match_bid(&row.text(), &profile) {
                    kept += 1;
                    *hits.entry(term).or_default() += 1;
                }
            }
            let today = rows.iter().filter(|row| row.published.starts_with("11-Oct-2026")).count();
            let rate = (kept as f64) / (rows.len() as f64) * 100.0;
            println!("{name}: {} scanned, {kept} kept, {rate:.1}% match", rows.len());
            for term in &profile.keep {
                println!("  {term}: {}", hits.get(term).copied().unwrap_or(0));
            }
            for state in watch {
                let n = rows.iter().filter(|row| row.buyer == state).count();
                if n > 0 {
                    println!("  state {state}: {n}");
                }
            }
            println!("  published today: {today}");
            if name == "CPPP states" {
                state_link = rows.first().map(|row| row.link.clone());
            }
        }
        print_gates(state_link.as_deref()).await;
    }

    async fn print_gates(detail: Option<&str>) {
        let http = reqwest::Client::builder()
            .user_agent(super::USER_AGENT)
            .timeout(std::time::Duration::from_secs(40))
            .build()
            .expect("client");
        let detail = detail.unwrap_or("");
        let body = http
            .get(detail)
            .header("Referer", "https://eprocure.gov.in/cppp/latestactivetendersnew/mmpdata")
            .send()
            .await
            .map(|response| response.text())
            .ok();
        let text = match body {
            Some(pending) => pending.await.unwrap_or_default(),
            None => String::new(),
        };
        let lower_detail = text.to_lowercase();
        let schedule = lower_detail.contains("schedule of") || lower_detail.contains("bill of quantity");
        let invalid = text.contains("Invalid Url");
        println!(
            "gate detail: {}",
            if schedule {
                "pass, schedule text"
            } else if invalid {
                "pass, stays Invalid Url"
            } else if text.is_empty() {
                "fail, no body"
            } else {
                "fail, no schedule text"
            }
        );
        for (label, url) in [
            ("IREPS quick-search", "https://www.ireps.gov.in/eps/anonymQuickSearch.do?searchParam=showPage&language=en"),
            ("IREPS advance-search", "https://www.ireps.gov.in/eps/anonymSearch.do?language=en&searchParam=showPage"),
        ] {
            let page = http.get(url).send().await.ok();
            let page = match page {
                Some(response) => response.text().await.unwrap_or_default(),
                None => String::new(),
            };
            let lower = page.to_lowercase();
            let challenge = lower.contains("captcha") || lower.contains("otp");
            let rows = super::id_pattern().is_match(&page);
            println!(
                "gate {label}: {}",
                if rows {
                    "pass, rows"
                } else if challenge {
                    "pass, challenge"
                } else if page.is_empty() {
                    "fail, no body"
                } else {
                    "fail, no rows"
                }
            );
        }
        let home = http.get("https://etender.cpwd.gov.in/").send().await.ok();
        let home = match home {
            Some(response) => response.text().await.unwrap_or_default(),
            None => String::new(),
        };
        let script = http
            .get("https://etender.cpwd.gov.in/resources/jsFiles/temp.test.js")
            .send()
            .await
            .ok();
        let script = match script {
            Some(response) => response.text().await.unwrap_or_default(),
            None => String::new(),
        };
        let _ = home;
        let tender_url = script.contains("eprocure") || script.contains("etender.cpwd");
        println!(
            "gate CPWD script: {}",
            if script.is_empty() {
                "fail, script missing"
            } else if tender_url {
                "pass, names a tender URL"
            } else {
                "pass, names no tender URL"
            }
        );
    }
}

fn tag_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?s)<[^>]+>").expect("tag"))
}

#[cfg(test)]
mod tests {
    use super::parse_list;

    const ROW: &str = r#"
<tr style="border-bottom: 1px solid #ffffff;">
<td style="text-align: left;">1.</td>
<td style="text-align: left;">11-Oct-2026 01:00 PM</td>
<td style="text-align: left;">16-Oct-2026 12:00 PM</td>
<td style="text-align: left;">16-Oct-2026 12:30 PM</td>
<td style="text-align: left;"><a href="https://eprocure.gov.in/cppp/tendersfullviewmmp/abc" title="External Url">Construction of the Piprahwa connecting road</a>/3626/2026_CEUFZ_1195512_1</td>
<td style="text-align: left;">Uttar Pradesh</td>
<td style="text-align: left;">--</td>
</tr>
<tr>
<td>2.</td><td>11-Oct-2026</td><td>20-Oct-2026</td><td>21-Oct-2026</td>
<td>Complete Cleaning and Waste Collection work/2026_DLB_603778_1</td>
<td>Rajasthan</td><td>--</td>
</tr>
"#;

    #[test]
    fn reads_a_state_row_and_a_plain_title() {
        let notices = parse_list("CPPP states", ROW);
        assert_eq!(notices.len(), 2);
        assert_eq!(notices[0].id, "2026_CEUFZ_1195512_1");
        assert_eq!(notices[0].title, "Construction of the Piprahwa connecting road");
        assert_eq!(notices[0].buyer, "Uttar Pradesh");
        assert_eq!(notices[0].published, "11-Oct-2026 01:00 PM");
        assert_eq!(notices[0].closing, "16-Oct-2026 12:00 PM");
        assert!(notices[0].link.contains("tendersfullviewmmp"));
        assert_eq!(notices[1].buyer, "Rajasthan");
        assert!(notices[1].title.contains("Waste Collection"));
    }

    const PLASTER: &str = r#"
<tr><td>1.</td><td>11-Oct-2026 09:00 AM</td><td>20-Oct-2026 03:00 PM</td><td>20-Oct-2026 03:30 PM</td>
<td><a href="https://eprocure.gov.in/cppp/tendersfullview/abc">Providing 12 mm cement plaster</a>/2026_MES_793307_1</td>
<td>Military Engineer Services</td><td>--</td></tr>
<tr><td>2.</td><td>11-Oct-2026 09:00 AM</td><td>20-Oct-2026 03:00 PM</td><td>20-Oct-2026 03:30 PM</td>
<td>Plaster of Paris Bandage for cement plaster kits/2026_MES_793308_1</td>
<td>Military Engineer Services</td><td>--</td></tr>
"#;

    fn navoday() -> crate::profile::Profile {
        serde_json::from_str(include_str!("../profiles/navoday-sciences.json")).unwrap()
    }

    #[test]
    fn a_cleaning_title_matches_nothing() {
        let notices = parse_list("CPPP states", ROW);
        let profile = navoday();
        assert!(crate::profile::match_bid(&notices[1].text(), &profile).is_none());
        assert!(notices[1].text().contains("Waste Collection"));
    }

    #[test]
    fn a_plaster_title_matches_and_a_bandage_title_is_dropped() {
        let notices = parse_list("CPPP central", PLASTER);
        let profile = navoday();
        assert_eq!(
            crate::profile::match_bid(&notices[0].text(), &profile).as_deref(),
            Some("cement plaster")
        );
        assert!(crate::profile::match_bid(&notices[1].text(), &profile).is_none());
        assert!(notices[0].link.starts_with("https://"));
        assert!(!notices[0].text().contains('@'));
    }

    #[test]
    fn the_same_id_on_two_lists_counts_once() {
        let mut seen = std::collections::HashSet::new();
        let one = r#"<tr><td>1.</td><td>11-Oct-2026</td><td>20-Oct-2026</td><td>21-Oct-2026</td><td>12 mm cement plaster/2026_MES_793307_1</td><td>MES</td><td>--</td></tr>"#;
        let fresh = parse_list("CPPP states", one)
            .into_iter()
            .chain(parse_list("CPPP central", one))
            .filter(|notice| seen.insert(notice.id.clone()))
            .count();
        assert_eq!(fresh, 1);
    }
}
