use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Profile {
    pub company: String,
    pub does: String,
    pub keep: Vec<String>,
    pub drop: Vec<String>,
    pub phrases: Vec<Phrase>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BidStatus {
    #[default]
    Ongoing,
    Awarded,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Phrase {
    pub query: String,
    #[serde(default = "default_pages")]
    pub pages: u32,
    #[serde(default)]
    pub status: BidStatus,
}

fn default_pages() -> u32 {
    2
}

impl Profile {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let profile: Self = serde_json::from_str(&raw).map_err(|e| format!("profile {}: {e}", path.display()))?;
        profile.check()
    }

    pub fn from_does(company: &str, does: &str) -> Result<Self, String> {
        let keep: Vec<String> = does
            .split([',', ';'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        let phrases = keep
            .iter()
            .map(|query| Phrase {
                query: query.clone(),
                pages: 2,
                status: BidStatus::Ongoing,
            })
            .collect();
        let profile = Self {
            company: company.to_string(),
            does: does.trim().to_string(),
            keep,
            drop: Vec::new(),
            phrases,
        };
        profile.check()
    }

    fn check(self) -> Result<Self, String> {
        if self.company.trim().is_empty() {
            return Err("profile needs a company name".into());
        }
        if self.keep.is_empty() || self.phrases.is_empty() {
            return Err("profile needs keep terms and search phrases".into());
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Keep(String),
    Drop(String),
}

pub fn judge(text: &str, profile: &Profile) -> Option<Verdict> {
    if let Some(term) = find_term(text, &profile.drop) {
        return Some(Verdict::Drop(term));
    }
    find_term(text, &profile.keep).map(Verdict::Keep)
}

#[cfg(test)]
pub fn match_bid(text: &str, profile: &Profile) -> Option<String> {
    match judge(text, profile) {
        Some(Verdict::Keep(term)) => Some(term),
        _ => None,
    }
}

pub fn term_hit(text: &str, term: &str) -> bool {
    let text = text.to_lowercase();
    let term = term.trim().to_lowercase();
    if term.is_empty() {
        return false;
    }
    let mut rest = text.as_str();
    let mut offset = 0usize;
    while let Some(at) = rest.find(&term) {
        let start = offset + at;
        let end = start + term.len();
        if boundary_before(&text, start) && boundary_after(&text, end) {
            return true;
        }
        let next = at + term.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        offset += next;
        rest = &rest[next..];
    }
    false
}

fn find_term(text: &str, terms: &[String]) -> Option<String> {
    terms.iter().find(|term| term_hit(text, term)).cloned()
}

fn boundary_before(text: &str, start: usize) -> bool {
    text[..start].chars().next_back().map(|c| !c.is_alphanumeric()).unwrap_or(true)
}

fn boundary_after(text: &str, end: usize) -> bool {
    text[end..].chars().next().map(|c| !c.is_alphanumeric()).unwrap_or(true)
}

pub fn snippet_around(text: &str, term: &str) -> String {
    let lower = text.to_lowercase();
    let needle = term.to_lowercase();
    let (start, end) = match lower.find(&needle) {
        Some(at) => {
            let start = at.saturating_sub(50);
            let end = (at + needle.len() + 90).min(text.len());
            (start, end)
        }
        None => (0, text.len().min(180)),
    };
    text[start..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn navoday() -> Profile {
        Profile::from_does(
            "Navoday Sciences",
            "cement plaster, plastering, wall putty",
        )
        .unwrap()
    }

    #[test]
    fn keeps_a_bill_that_names_the_work() {
        let profile = navoday();
        let text = "Supply and making 15 mm thick cement plaster on inside wall";
        assert_eq!(match_bid(text, &profile).as_deref(), Some("cement plaster"));
    }

    #[test]
    fn drops_a_listed_false_match() {
        let mut profile = navoday();
        profile.drop.push("plaster of paris bandage".into());
        let text = "Plaster of Paris Bandage for cement plaster kits";
        assert_eq!(match_bid(text, &profile), None);
    }

    #[test]
    fn another_company_uses_its_own_words() {
        let profile = Profile::from_does("Northwind Pumps", "submersible pump, dewatering").unwrap();
        assert_eq!(
            match_bid("Hire of submersible pump for basement", &profile).as_deref(),
            Some("submersible pump")
        );
        assert_eq!(match_bid("12 mm cement plaster", &profile), None);
    }

    #[test]
    fn siem_misses_siemens_and_a_real_siem_line_is_kept() {
        let profile = Profile {
            company: "Varonis".into(),
            does: "data security".into(),
            keep: vec!["siem".into(), "soar".into()],
            drop: vec!["dlp support".into(), "cctv".into()],
            phrases: vec![Phrase {
                query: "siem".into(),
                pages: 1,
                status: BidStatus::Ongoing,
            }],
        };
        assert_eq!(judge("Supply SIEMENS HT Breaker", &profile), None);
        assert_eq!(
            judge("SIEM and SOAR Software", &profile),
            Some(Verdict::Keep("siem".into()))
        );
        assert_eq!(
            judge("2 years DLP support for CCTV Video Surveillance", &profile),
            Some(Verdict::Drop("dlp support".into()))
        );
    }
}
