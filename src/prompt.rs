const TEMPLATE: &str = include_str!("../prompts/company-watch.md");

pub fn fill(company: &str, does: &str) -> String {
    TEMPLATE
        .replace("{{company}}", company.trim())
        .replace("{{does}}", does.trim())
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn names_the_company_and_the_profile_fields() {
        let prompt = fill("Varonis", "Data security for file shares and Microsoft 365");
        assert!(prompt.contains("Varonis"));
        assert!(prompt.contains("Data security for file shares and Microsoft 365"));
        assert!(prompt.contains("`keep`"));
        assert!(prompt.contains("`drop`"));
        assert!(prompt.contains("`phrases`"));
    }
}
