use regex::Regex;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Extractor {
    pub source: String,
    pub url: String,
    #[serde(default = "method_get")]
    pub method: String,
    pub row: Option<String>,
    #[serde(default)]
    pub contains: Vec<String>,
    pub require: Option<String>,
    #[serde(default)]
    pub fill: Fill,
}

fn method_get() -> String {
    "GET".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Fill {
    pub title: Option<String>,
    pub place: Option<String>,
    pub date: Option<String>,
    pub asset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntelRow {
    pub source: String,
    pub title: String,
    pub place: String,
    pub date: String,
    pub asset: String,
    pub url: String,
}

impl Extractor {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let extractor: Self = serde_json::from_str(&raw).map_err(|e| format!("extractor {}: {e}", path.display()))?;
        if !extractor.method.eq_ignore_ascii_case("GET") {
            return Err("extractor method must be GET".into());
        }
        if extractor.source.trim().is_empty() || extractor.url.trim().is_empty() {
            return Err("extractor needs a source and a url".into());
        }
        Ok(extractor)
    }
}

pub fn apply(extractor: &Extractor, body: &str) -> Result<Vec<IntelRow>, String> {
    let text = visible_text(body);
    let mut rows = Vec::new();
    if let Some(pattern) = &extractor.row {
        let re = Regex::new(pattern).map_err(|e| format!("extractor row regex: {e}"))?;
        for caps in re.captures_iter(&text) {
            let place = named(&caps, "place").unwrap_or_default();
            let date = named(&caps, "date").unwrap_or_default();
            let asset = named(&caps, "asset").or_else(|| extractor.fill.asset.clone()).unwrap_or_default();
            let title = named(&caps, "title").unwrap_or_else(|| {
                fill_template(extractor.fill.title.as_deref().unwrap_or("{place} {date}"), &place, &date, &asset)
            });
            push_row(&mut rows, extractor, title, place, date, asset);
        }
    }
    for needle in &extractor.contains {
        if text.to_lowercase().contains(&needle.to_lowercase()) {
            let asset = extractor.fill.asset.clone().unwrap_or_else(|| needle.clone());
            let title = fill_template(
                extractor.fill.title.as_deref().unwrap_or(needle),
                extractor.fill.place.as_deref().unwrap_or(""),
                extractor.fill.date.as_deref().unwrap_or(""),
                &asset,
            );
            let title = if title == *needle { needle.clone() } else { title.replace("{item}", needle) };
            push_row(
                &mut rows,
                extractor,
                title,
                extractor.fill.place.clone().unwrap_or_default(),
                extractor.fill.date.clone().unwrap_or_default(),
                asset,
            );
        }
    }
    if rows.is_empty() {
        if let Some(need) = &extractor.require {
            if text.to_lowercase().contains(&need.to_lowercase()) {
                push_row(
                    &mut rows,
                    extractor,
                    extractor.fill.title.clone().unwrap_or_else(|| need.clone()),
                    extractor.fill.place.clone().unwrap_or_default(),
                    extractor.fill.date.clone().unwrap_or_default(),
                    extractor.fill.asset.clone().unwrap_or_default(),
                );
            }
        }
    }
    if rows.is_empty() && is_blocked(body) {
        push_row(&mut rows, extractor, "blocked".into(), String::new(), String::new(), "blocked".into());
    }
    Ok(rows)
}

fn push_row(rows: &mut Vec<IntelRow>, extractor: &Extractor, title: String, place: String, date: String, asset: String) {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() || rows.iter().any(|row| row.title == title) {
        return;
    }
    rows.push(IntelRow {
        source: extractor.source.clone(),
        title,
        place: place.trim().to_string(),
        date: date.trim().to_string(),
        asset: asset.trim().to_string(),
        url: extractor.url.clone(),
    });
}

fn named(caps: &regex::Captures, name: &str) -> Option<String> {
    caps.name(name).map(|m| m.as_str().trim().to_string()).filter(|s| !s.is_empty())
}

fn fill_template(template: &str, place: &str, date: &str, asset: &str) -> String {
    template
        .replace("{place}", place)
        .replace("{date}", date)
        .replace("{asset}", asset)
}

pub fn visible_text(body: &str) -> String {
    let without_script = Regex::new(r"(?is)<script[\s\S]*?</script>|<style[\s\S]*?</style>")
        .expect("strip")
        .replace_all(body, " ");
    let text = Regex::new(r"(?s)<[^>]+>").expect("tags").replace_all(&without_script, " ");
    let text = text.replace("&nbsp;", " ").replace("&amp;", "&").replace("&#39;", "'");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_blocked(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("captcha") || lower.contains("otp") || lower.contains("type=\"password\"") || lower.contains("type='password'")
}

pub fn open_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS intel (
            source TEXT NOT NULL,
            title TEXT NOT NULL,
            place TEXT NOT NULL DEFAULT '',
            date TEXT NOT NULL DEFAULT '',
            asset TEXT NOT NULL DEFAULT '',
            url TEXT NOT NULL,
            fetched_at TEXT NOT NULL,
            PRIMARY KEY (source, title)
        );",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn upsert(conn: &Connection, row: &IntelRow, fetched_at: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO intel (source, title, place, date, asset, url, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(source, title) DO UPDATE SET
            place = excluded.place,
            date = excluded.date,
            asset = excluded.asset,
            url = excluded.url,
            fetched_at = excluded.fetched_at",
        params![row.source, row.title, row.place, row.date, row.asset, row.url, fetched_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ask_place(conn: &Connection, place: &str) -> Result<Vec<IntelRow>, String> {
    let mut stmt = conn
        .prepare("SELECT source, title, place, date, asset, url FROM intel WHERE place LIKE ?1 ORDER BY date, title")
        .map_err(|e| e.to_string())?;
    let needle = format!("%{}%", place);
    let rows = stmt
        .query_map(params![needle], |row| {
            Ok(IntelRow {
                source: row.get(0)?,
                title: row.get(1)?,
                place: row.get(2)?,
                date: row.get(3)?,
                asset: row.get(4)?,
                url: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c1() -> Extractor {
        Extractor {
            source: "c1india".into(),
            url: "https://eauctions.c1india.com/".into(),
            method: "GET".into(),
            row: Some(r"(?P<place>[A-Z][A-Za-z]+(?: & [A-Z][A-Za-z]+)?) (?P<date>\d{2}-[A-Za-z]{3}-\d{4})".into()),
            contains: vec![],
            require: None,
            fill: Fill { title: Some("{place} {date}".into()), place: None, date: None, asset: Some("auction".into()) },
        }
    }

    #[test]
    fn lifts_city_and_date_beside_a_login_form() {
        let html = r#"<div>Auction Notice &nbsp; Asaoti 13-Oct-2026 &nbsp; Chennai &amp; Chittoor 13-Oct-2026</div>
            <input type="password" name="password">"#;
        let rows = apply(&c1(), html).unwrap();
        assert!(rows.iter().any(|row| row.place == "Asaoti" && row.date == "13-Oct-2026"));
        assert!(rows.iter().any(|row| row.place == "Chennai & Chittoor" && row.asset == "auction"));
        assert!(rows.iter().all(|row| row.title != "blocked"));
    }

    #[test]
    fn stores_named_families_from_the_page() {
        let extractor = Extractor {
            source: "mstc".into(),
            url: "https://www.mstcecommerce.com/".into(),
            method: "GET".into(),
            row: None,
            contains: vec!["Scrap".into(), "Coal".into(), "Linkage".into()],
            require: None,
            fill: Fill { title: Some("{item}".into()), place: Some(String::new()), date: Some(String::new()), asset: Some("auction-family".into()) },
        };
        let rows = apply(&extractor, "<nav>Scrap & Customs Goods Coal linkage e-Auction</nav>").unwrap();
        let titles: Vec<_> = rows.iter().map(|row| row.title.as_str()).collect();
        assert_eq!(titles, vec!["Scrap", "Coal", "Linkage"]);
    }

    #[test]
    fn one_notice_when_the_required_words_are_present() {
        let extractor = Extractor {
            source: "jda-rajasthan".into(),
            url: "https://jdaservice.rajasthan.gov.in/auction".into(),
            method: "GET".into(),
            row: None,
            contains: vec![],
            require: Some("osdrm.jda@rajasthan.gov.in".into()),
            fill: Fill {
                title: Some("E-auction sale of development authority property osdrm.jda@rajasthan.gov.in".into()),
                place: Some("Rajasthan".into()),
                date: Some(String::new()),
                asset: Some("land".into()),
            },
        };
        let rows = apply(&extractor, "<p>E-Auction Sale of Properties. E-Mail : osdrm.jda@rajasthan.gov.in</p>").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].place, "Rajasthan");
        assert_eq!(rows[0].asset, "land");
    }

    #[test]
    fn a_challenge_with_no_rows_stores_one_blocked_row() {
        let extractor = Extractor {
            source: "eauction".into(),
            url: "https://eauction.gov.in/eAuction/app".into(),
            method: "GET".into(),
            row: None,
            contains: vec!["Scrap".into()],
            require: None,
            fill: Fill::default(),
        };
        let rows = apply(&extractor, "<form>Enter captcha <input name='captchaText'></form>").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "blocked");
        assert_eq!(rows[0].asset, "blocked");
    }

    #[test]
    fn the_same_source_and_title_updates_one_row() {
        let dir = std::env::temp_dir().join(format!("gem-probe-intel-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("intel.db");
        let conn = open_db(&path).unwrap();
        let row = IntelRow {
            source: "c1india".into(),
            title: "Chennai 13-Oct-2026".into(),
            place: "Chennai".into(),
            date: "13-Oct-2026".into(),
            asset: "auction".into(),
            url: "https://eauctions.c1india.com/".into(),
        };
        upsert(&conn, &row, "2026-10-11T00:00:00Z").unwrap();
        let mut again = row.clone();
        again.date = "14-Oct-2026".into();
        upsert(&conn, &again, "2026-10-12T00:00:00Z").unwrap();
        let found = ask_place(&conn, "Chennai").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].date, "14-Oct-2026");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
