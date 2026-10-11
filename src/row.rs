#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    Bill,
    Title,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    Inside,
    Outside,
    Undated,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub source: String,
    pub id: String,
    pub buyer: String,
    pub title: String,
    pub text: String,
    pub end: String,
    pub link: String,
    pub matched: Option<String>,
    pub dropped: Option<String>,
    pub coverage: Coverage,
    pub account: Option<String>,
    pub doc_id: Option<u64>,
}

impl Row {
    pub fn key(&self) -> String {
        format!("{}:{}", self.source, self.id)
    }
}

#[derive(Clone, Debug)]
pub struct PhraseStat {
    pub query: String,
    pub in_text: u64,
    pub in_index: u64,
    pub missing: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FeedStat {
    pub name: String,
    pub rows: usize,
    pub error: Option<String>,
    pub detail: Option<String>,
}

pub fn when(today: &str, end: &str, within: u32) -> When {
    let end: String = end.chars().take(10).collect();
    let Some(days) = days_between(today, &end) else {
        return When::Undated;
    };
    if (0..=within as i64).contains(&days) {
        When::Inside
    } else {
        When::Outside
    }
}

pub fn today_ymd() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn render(
    company: &str,
    today: &str,
    within: u32,
    rows: &[Row],
    phrases: &[PhraseStat],
    feeds: &[FeedStat],
) -> String {
    let act = select(rows, today, within, When::Inside, Coverage::Bill, true);
    let undated = select(rows, today, within, When::Undated, Coverage::Bill, true);
    let titles = rows
        .iter()
        .filter(|row| row.coverage == Coverage::Title && row.matched.is_some())
        .collect::<Vec<_>>();
    let lookalikes = rows.iter().filter(|row| row.dropped.is_some()).collect::<Vec<_>>();
    let mut on_list = act.clone();
    on_list.extend(undated.iter().copied());
    on_list.extend(titles.iter().copied());
    on_list.retain(|row| row.account.is_some());

    let mut out = format!("# {company}\n\n## Act this week\n\n");
    push_rows(&mut out, &act);
    out.push_str("\n### No close date\n\n");
    push_rows(&mut out, &undated);
    out.push_str("\n## On your account list\n\n");
    if on_list.is_empty() {
        out.push_str("None of these buyers are on the account list.\n");
    } else {
        for row in on_list {
            out.push_str(&format!(
                "- {} — {} — {}\n",
                row.id,
                row.account.as_deref().unwrap_or(""),
                row.buyer
            ));
        }
    }
    out.push_str("\n## Lookalikes dropped\n\n");
    if lookalikes.is_empty() {
        out.push_str("None.\n");
    } else {
        for row in lookalikes {
            out.push_str(&format!(
                "- {} — {} — {}\n",
                row.id,
                row.dropped.as_deref().unwrap_or(""),
                clip(&row.text)
            ));
        }
    }
    out.push_str("\n## What the feeds did\n\n");
    for phrase in phrases {
        if let Some(err) = &phrase.error {
            out.push_str(&format!("- {}: fail, {err}\n", phrase.query));
        } else if phrase.missing {
            out.push_str(&format!("- {}: no results\n", phrase.query));
        } else {
            out.push_str(&format!(
                "- {}: {} in text / {} in index\n",
                phrase.query, phrase.in_text, phrase.in_index
            ));
        }
    }
    for feed in feeds {
        match &feed.error {
            Some(err) => out.push_str(&format!("- {}: {} rows retained; fail, {err}\n", feed.name, feed.rows)),
            None => out.push_str(&format!("- {}: {} rows, title only\n", feed.name, feed.rows)),
        }
    }
    for feed in feeds {
        if let Some(detail) = &feed.detail {
            out.push_str(&format!("- {} coverage: {detail}\n", feed.name));
        }
    }
    out.push_str("\n### Title matches\n\n");
    if titles.is_empty() {
        out.push_str("None.\n");
    } else {
        for row in titles {
            out.push_str(&format!(
                "- {} — {} — {} — {}\n",
                row.source,
                row.id,
                row.matched.as_deref().unwrap_or(""),
                row.title
            ));
        }
    }
    out
}

fn select<'a>(
    rows: &'a [Row],
    today: &str,
    within: u32,
    when_kind: When,
    coverage: Coverage,
    kept: bool,
) -> Vec<&'a Row> {
    rows.iter()
        .filter(|row| row.coverage == coverage)
        .filter(|row| if kept { row.matched.is_some() } else { row.dropped.is_some() })
        .filter(|row| when(today, &row.end, within) == when_kind)
        .collect()
}

fn push_rows(out: &mut String, rows: &[&Row]) {
    if rows.is_empty() {
        out.push_str("None.\n");
        return;
    }
    for row in rows {
        out.push_str(&format!(
            "- {} — {} — closes {} — {} — {}\n",
            row.id,
            row.buyer,
            row.end,
            row.matched.as_deref().unwrap_or(""),
            row.link
        ));
    }
}

fn clip(text: &str) -> String {
    let clipped: String = text.chars().take(140).collect();
    clipped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn days_between(today: &str, end: &str) -> Option<i64> {
    Some(ymd_to_days(end)? - ymd_to_days(today)?)
}

fn ymd_to_days(ymd: &str) -> Option<i64> {
    let mut parts = ymd.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = (year - era * 400) as u64;
    let month_prime = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * month_prime as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, end: &str, coverage: Coverage, matched: Option<&str>, dropped: Option<&str>) -> Row {
        Row {
            source: if coverage == Coverage::Bill { "GeM" } else { "Rajasthan" }.into(),
            id: id.into(),
            buyer: "Department of Space".into(),
            title: "board title".into(),
            text: "bill text".into(),
            end: end.into(),
            link: "https://bidplus.gem.gov.in/showbidDocument/1".into(),
            matched: matched.map(str::to_string),
            dropped: dropped.map(str::to_string),
            coverage,
            account: None,
            doc_id: None,
        }
    }

    #[test]
    fn failed_feed_retains_count_and_coverage() {
        let feeds = vec![FeedStat { name: "Portal".into(), rows: 7, error: Some("timeout".into()),
                                  detail: Some("partial or unresolved".into()) }];
        let page = render("Company", "2026-10-11", 14, &[], &[], &feeds);
        assert!(page.contains("7 rows retained; fail, timeout"));
        assert!(page.contains("Portal coverage: partial or unresolved"));
    }

    #[test]
    fn a_date_three_days_out_is_inside_and_twenty_is_outside() {
        assert_eq!(when("2026-10-11", "2026-10-14", 14), When::Inside);
        assert_eq!(when("2026-10-11", "2026-10-31", 14), When::Outside);
        assert_eq!(when("2026-10-11", "", 14), When::Undated);
    }

    #[test]
    fn the_page_names_the_four_parts_and_a_title_only_feed() {
        let rows = vec![
            row("GEM/1", "2026-10-14", Coverage::Bill, Some("dlp"), None),
            row("2026_PWD_1", "11-Oct-2026", Coverage::Title, Some("dlp"), None),
        ];
        let phrases = vec![PhraseStat {
            query: "data loss prevention".into(),
            in_text: 0,
            in_index: 285,
            missing: false,
            error: None,
        }];
        let feeds = vec![FeedStat {
            name: "Rajasthan".into(),
            rows: 10,
            error: None,
            detail: None,
        }];
        let page = render("Varonis", "2026-10-11", 14, &rows, &phrases, &feeds);
        assert!(page.contains("## Act this week"));
        assert!(page.contains("## On your account list"));
        assert!(page.contains("## Lookalikes dropped"));
        assert!(page.contains("## What the feeds did"));
        assert!(page.contains("0 in text / 285 in index"));
        assert!(page.contains("10 rows, title only"));
        let act = page.split("## On your account list").next().unwrap();
        assert!(act.contains("GEM/1"));
        assert!(!act.contains("2026_PWD_1"));
        assert!(page.contains("### Title matches"));
        assert!(page.contains("2026_PWD_1"));
    }
}
