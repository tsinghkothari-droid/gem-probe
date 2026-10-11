use crate::profile::term_hit;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Account {
    pub legal_name: String,
    pub short_name: String,
    pub state: String,
    pub tier: u8,
}

pub fn load(path: &Path) -> Result<Vec<Account>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("accounts {}: {e}", path.display()))
}

pub fn match_buyer(buyer: &str, accounts: &[Account]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for account in accounts {
        for raw in [&account.legal_name, &account.short_name] {
            let Some(name) = significant_name(&strip_note(raw)) else { continue };
            if term_hit(buyer, &name) {
                if best.as_ref().map(|(len, _)| name.len() > *len).unwrap_or(true) {
                    best = Some((
                        name.len(),
                        format!("{} ({}, tier {})", account.legal_name, account.state, account.tier),
                    ));
                }
            }
        }
    }
    best.map(|(_, name)| name)
}

fn strip_note(raw: &str) -> String {
    match raw.find('(') {
        Some(at) => raw[..at].trim().to_string(),
        None => raw.to_string(),
    }
}

fn significant_name(raw: &str) -> Option<String> {
    let spaced: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let name = spaced
        .split_whitespace()
        .filter(|token| !matches!(*token, "limited" | "ltd" | "pvt" | "private"))
        .collect::<Vec<_>>()
        .join(" ");
    if name.chars().count() >= 8 { Some(name) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Account> {
        vec![Account {
            legal_name: "Gujarat Informatics Limited".into(),
            short_name: "GIL".into(),
            state: "GJ".into(),
            tier: 1,
        }]
    }

    #[test]
    fn matches_the_legal_name_and_skips_the_state_word() {
        let accounts = sample();
        assert_eq!(
            match_buyer("Gujarat Informatics Limited, Gandhinagar", &accounts).as_deref(),
            Some("Gujarat Informatics Limited (GJ, tier 1)")
        );
        assert_eq!(match_buyer("Department of Science Technology Gujarat", &accounts), None);
    }

}
