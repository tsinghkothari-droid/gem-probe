use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

const LIST_PAGE: &str = "https://bidplus.gem.gov.in/all-bids";
const DATA_URL: &str = "https://bidplus.gem.gov.in/all-bids-data";
const USER_AGENT: &str = "gem-probe/0.1 (public BidPlus search; +https://github.com/tsinghkothari-droid/gem-probe)";

pub struct Gem {
    http: Client,
    csrf: String,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    response: Inner,
}

#[derive(Debug, Deserialize)]
struct Inner {
    response: Page,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Page {
    #[serde(rename = "numFound")]
    pub num_found: u64,
    #[serde(default)]
    pub docs: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct Bid {
    pub number: String,
    pub id: Option<u64>,
    pub title: String,
    pub ministry: String,
    pub department: String,
    pub text: String,
    pub end: String,
}

impl Bid {
    pub fn pdf_url(&self) -> String {
        match self.id {
            Some(id) => format!("https://bidplus.gem.gov.in/showbidDocument/{id}"),
            None => String::new(),
        }
    }
}

impl Gem {
    pub async fn connect() -> Result<Self, String> {
        let http = Client::builder()
            .cookie_store(true)
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(40))
            .build()
            .map_err(|e| e.to_string())?;
        let html = http
            .get(LIST_PAGE)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let csrf = html
            .split("csrf_bd_gem_nk': '")
            .nth(1)
            .and_then(|rest| rest.split('\'').next())
            .filter(|token| !token.is_empty())
            .ok_or_else(|| "GeM page did not include a csrf token".to_string())?
            .to_string();
        Ok(Self { http, csrf })
    }

    pub async fn search(&self, query: &str, page: u32) -> Result<Page, String> {
        let payload = json!({
            "param": {"searchBid": query, "searchType": "fullText"},
            "filter": {
                "bidStatusType": "ongoing_bids",
                "byType": "all",
                "highBidValue": "",
                "byEndDate": {"from": "", "to": ""},
                "sort": "Bid-End-Date-Oldest"
            },
            "page": page
        });
        let response = self
            .http
            .post(DATA_URL)
            .header("Referer", LIST_PAGE)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&[
                ("payload", payload.to_string()),
                ("csrf_bd_gem_nk", self.csrf.clone()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(format!("no results endpoint for {query:?}"));
        }
        let response = response.error_for_status().map_err(|e| e.to_string())?;
        let envelope: Envelope = response.json().await.map_err(|e| e.to_string())?;
        Ok(envelope.response.response)
    }
}

pub fn bid_from_doc(doc: &Value) -> Bid {
    let number = first_text(doc.get("b_bid_number")).replace("\\/", "/");
    let id = doc
        .get("b_id")
        .and_then(first_number);
    let title = join_text(doc.get("bbt_title"));
    let mut category = join_text(doc.get("bd_category_name"));
    if category.is_empty() {
        category = join_text(doc.get("b_category_name"));
    }
    let text = format!("{title} {category}")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Bid {
        number,
        id,
        title,
        ministry: join_text(doc.get("ba_official_details_minName")),
        department: join_text(doc.get("ba_official_details_deptName")),
        text,
        end: first_text(doc.get("final_end_date_sort")).chars().take(10).collect(),
    }
}

fn first_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items.first().map(|v| first_text(Some(v))).unwrap_or_default(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn join_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| first_text(Some(item)))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        other => first_text(other),
    }
}

fn first_number(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::Array(items) => items.first().and_then(first_number),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}
