use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

const COLLECTOR: &str = include_str!("../organisation_tenders.py");
const PORTALS: &str = include_str!("../gepnic-portals.json");

#[derive(Clone, Debug, clap::Args)]
pub struct Options {
    #[arg(long = "portal", help = "Tender portal host to scan; repeat to select several. Default: all configured hosts.")]
    pub portals: Vec<String>,
    #[arg(long, default_value_t = 0, help = "Maximum organisations per portal; 0 scans all listed organisations.")]
    pub max_organisations: usize,
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u64).range(1..))]
    pub portal_timeout: u64,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..))]
    pub listing_pages: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self { portals: Vec::new(), max_organisations: 0, portal_timeout: 90, listing_pages: 2 }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Portal {
    host: String,
    url: String,
    organisation: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Notice {
    pub id: String,
    pub title: String,
    pub reference: String,
    pub buyer_chain: String,
    pub closing: String,
    pub published: String,
    pub opening: String,
    pub source_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Scan {
    pub organisation: String,
    #[serde(default)]
    pub advertised_count: Option<usize>,
    #[serde(default)]
    pub parsed_count: Option<usize>,
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Report {
    pub host: String,
    #[serde(default)]
    pub kind: String,
    pub source_url: String,
    pub checked_at: String,
    pub status: String,
    #[serde(default)]
    pub organisation_filter: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub organisations_listed: usize,
    #[serde(default)]
    pub organisations_selected: usize,
    #[serde(default)]
    pub organisations_matching: usize,
    #[serde(default)]
    pub advertised_active_tenders: Option<usize>,
    #[serde(default)]
    pub complete_active_snapshot: bool,
    #[serde(default)]
    pub complete_selected_snapshot: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub captcha_attempts: Vec<serde_json::Value>,
    pub scans: Vec<Scan>,
    pub tenders: Vec<Notice>,
}

impl Report {
    pub fn coverage(&self) -> String {
        if self.kind == "bihar" {
            let state = if self.complete_active_snapshot { "complete active listing" } else { "partial or unresolved" };
            return format!("{state}; {} records, advertised total {}", self.tenders.len(),
                self.advertised_active_tenders.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into()));
        }
        if !self.kind.is_empty() {
            let state = if self.complete_selected_snapshot { "complete selected listing" } else { "partial public listing" };
            return format!("{state}; {} records; {}", self.tenders.len(), self.scope);
        }
        let matched = self.scans.iter().filter(|s| s.status == "count_matches").count();
        let scope = if self.organisation_filter.is_empty() {
            "all organisations".to_string()
        } else {
            self.organisation_filter.clone()
        };
        let completeness = if self.complete_active_snapshot {
            "complete active listing"
        } else if !self.organisation_filter.is_empty() && self.complete_selected_snapshot {
            "complete selected buyer listing"
        } else {
            "partial or unresolved"
        };
        format!("{completeness}; {}/{} organisations scanned, {matched} counts matched; scope: {scope}",
                self.scans.len(), self.organisations_listed)
    }

    pub fn failure(&self) -> Option<String> {
        if let Some(error) = &self.error {
            return Some(error.clone());
        }
        if self.status != "records_returned" && self.status != "verified_empty" {
            let solver_error = self.captcha_attempts.last().and_then(|a| a.get("error")).and_then(|e| e.as_str());
            return Some(solver_error.map(|e| format!("{}: {e}", self.status)).unwrap_or_else(|| self.status.clone()));
        }
        let failures = self.scans.iter().filter(|s| !matches!(s.status.as_str(), "count_matches" | "page_limit" | "sample_only" | "unverified_total"))
            .map(|s| format!("{}: {}", s.organisation, s.error.as_deref().unwrap_or(&s.status)))
            .collect::<Vec<_>>();
        if failures.is_empty() { None } else { Some(failures.join("; ")) }
    }
}

pub fn select(options: &Options) -> Result<Vec<Portal>, String> {
    let portals: Vec<Portal> = serde_json::from_str(PORTALS).map_err(|e| e.to_string())?;
    for host in &options.portals {
        if !portals.iter().any(|p| p.host == *host) {
            return Err(format!("unknown tender portal {host}; use a hostname from gepnic-portals.json"));
        }
    }
    Ok(portals.into_iter().filter(|p| options.portals.is_empty() || options.portals.contains(&p.host)).collect())
}

pub async fn fetch(options: Options) -> Result<Vec<Report>, String> {
    let portals = select(&options)?;
    tokio::task::spawn_blocking(move || {
        let mut child = Command::new("python3")
            .args(["-c", COLLECTOR, "--batch", "--out", "-", "--transport", "curl", "--max-orgs"])
            .arg(options.max_organisations.to_string())
            .arg("--timeout").arg(options.portal_timeout.to_string())
            .arg("--listing-pages").arg(options.listing_pages.to_string())
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
            .spawn().map_err(|e| format!("start organisation collector (requires Python 3 and curl): {e}"))?;
        let input = serde_json::to_vec(&portals).map_err(|e| e.to_string())?;
        let mut stdin = child.stdin.take().ok_or("collector stdin unavailable")?;
        stdin.write_all(&input).map_err(|e| e.to_string())?;
        drop(stdin);
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("organisation collector exited with {}", output.status));
        }
        serde_json::from_slice(&output.stdout).map_err(|e| format!("invalid collector response: {e}"))
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_samples_keep_unknown_totals_without_a_false_failure() {
        let report: Report = serde_json::from_value(serde_json::json!({
            "host":"example.test", "kind":"ap_home", "scope":"Homepage only",
            "source_url":"https://example.test", "checked_at":"2026-10-11",
            "status":"records_returned", "scans":[{"organisation":"Homepage",
            "advertised_count":null, "parsed_count":5, "status":"sample_only"}], "tenders":[]
        })).unwrap();
        assert!(report.coverage().contains("partial public listing"));
        assert!(report.coverage().contains("Homepage only"));
        assert!(report.failure().is_none());
    }

    #[test]
    fn aliases_select_the_intended_buyers() {
        let options = Options { portals: vec!["eprocurebel.co.in".into()], ..Options::default() };
        let portals = select(&options).unwrap();
        assert_eq!(portals.len(), 1);
        assert_eq!(portals[0].organisation, "Bharat Electronics Limited");
        assert!(select(&Options { portals: vec!["unknown.test".into()], ..Options::default() }).is_err());
    }

    #[test]
    fn partial_scan_is_not_reported_as_complete() {
        let report: Report = serde_json::from_value(serde_json::json!({
            "host":"example.test", "source_url":"https://example.test", "checked_at":"2026-10-11",
            "status":"records_returned", "organisations_listed":10, "organisations_selected":1,
            "complete_selected_snapshot":true, "scans":[{"organisation":"A", "status":"count_matches"}],
            "tenders":[]
        })).unwrap();
        assert!(report.coverage().contains("partial or unresolved"));
        assert!(report.coverage().contains("1/10 organisations"));
    }
}
