mod accounts;
mod cppp;
mod extract;
mod filter;
mod gem;
mod gepnic;
mod organisation;
mod profile;
mod prompt;
mod row;

use accounts::Account;
use clap::{Parser, Subcommand};
use filter::{classify, snippet, Kind};
use gem::{bid_from_doc, Gem};
use profile::{judge, snippet_around, term_hit, BidStatus, Profile, Verdict};
use row::{Coverage, FeedStat, PhraseStat, Row, When};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

const NAVODAY_PROFILE: &str = include_str!("../profiles/navoday-sciences.json");

#[derive(Parser)]
#[command(name = "gem-probe", about = "Watch public GeM bids for the work a company actually sells")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Watch GeM, the open boards, and the CPPP title lists for one company.
    Run(RunArgs),
    /// Same as run. Kept so older commands still work.
    Watch(RunArgs),
    /// Navoday Sciences watch, from the profile shipped with this tool.
    Navoday {
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        state: Option<PathBuf>,
    },
    /// Open every public feed once and print whether it returned rows.
    Sample,
    /// Collect active GePNIC and Bihar listings with coverage counts.
    Organisations {
        #[command(flatten)]
        options: organisation::Options,
        #[arg(long)]
        out: PathBuf,
    },
    /// Print the system prompt that turns a company description into a search profile.
    Prompt {
        #[arg(long)]
        company: String,
        #[arg(long)]
        does: String,
    },
    /// Read the six GePNIC by-date pages that already list tender rows.
    Boards {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Run an AI-written extractor and store the rows.
    Fetch {
        #[arg(long)]
        extractor: PathBuf,
        #[arg(long, default_value = "intel.db")]
        db: PathBuf,
    },
    /// Read stored rows for a place.
    Ask {
        #[arg(long)]
        place: String,
        #[arg(long, default_value = "intel.db")]
        db: PathBuf,
    },
    /// Run one search phrase and keep rows whose bill text matches that kind.
    Search {
        query: String,
        #[arg(long, default_value_t = KindArg::Plaster)]
        kind: KindArg,
        #[arg(long, default_value_t = 3)]
        pages: u32,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(clap::Args)]
struct RunArgs {
    #[command(flatten)]
    organisations: organisation::Options,
    #[arg(long)]
    profile: Option<PathBuf>,
    #[arg(long)]
    company: Option<String>,
    #[arg(long)]
    does: Option<String>,
    #[arg(long)]
    accounts: Option<PathBuf>,
    /// Keep bill matches that close within this many days.
    #[arg(long, default_value_t = 14)]
    within: u32,
    #[arg(long)]
    out: Option<PathBuf>,
    /// Bid numbers already reported. New bids are appended after the run.
    #[arg(long)]
    state: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum KindArg {
    Plaster,
    Masonry,
    House,
}

impl std::str::FromStr for KindArg {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plaster" => Ok(Self::Plaster),
            "masonry" => Ok(Self::Masonry),
            "house" => Ok(Self::House),
            other => Err(format!("unknown kind {other}; use plaster, masonry, or house")),
        }
    }
}

impl std::fmt::Display for KindArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Plaster => "plaster",
            Self::Masonry => "masonry",
            Self::House => "house",
        })
    }
}

impl From<KindArg> for Kind {
    fn from(value: KindArg) -> Self {
        match value {
            KindArg::Plaster => Kind::Plaster,
            KindArg::Masonry => Kind::Masonry,
            KindArg::House => Kind::House,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Seen {
    bids: Vec<String>,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("gem-probe: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Search { query, kind, pages, out } => {
            search_once(query, kind.into(), pages, out).await
        }
        Command::Navoday { out, state } => {
            let profile: Profile = serde_json::from_str(NAVODAY_PROFILE).map_err(|e| e.to_string())?;
            watch(profile, &[], 14, out, state, organisation::Options::default()).await
        }
        Command::Sample => sample().await,
        Command::Organisations { options, out } => {
            let reports = organisation::fetch(options).await?;
            if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let file = File::create(&out).map_err(|e| e.to_string())?;
            serde_json::to_writer_pretty(file, &reports).map_err(|e| e.to_string())?;
            for report in reports { eprintln!("{}: {}", report.host, report.coverage()); }
            Ok(())
        },
        Command::Boards { out } => boards(out).await,
        Command::Prompt { company, does } => {
            println!("{}", prompt::fill(&company, &does));
            Ok(())
        }
        Command::Fetch { extractor, db } => fetch_extractor(extractor, db).await,
        Command::Ask { place, db } => ask_intel(place, db),
        Command::Run(args) | Command::Watch(args) => {
            let profile = if let Some(path) = args.profile {
                Profile::load(&path)?
            } else {
                let company = args.company.ok_or("pass --profile, or --company and --does")?;
                let does = args.does.ok_or("pass --does with the work this company sells")?;
                Profile::from_does(&company, &does)?
            };
            let accounts = match args.accounts {
                Some(path) => accounts::load(&path)?,
                None => Vec::new(),
            };
            watch(profile, &accounts, args.within, args.out, args.state, args.organisations).await
        }
    }
}

async fn fetch_extractor(path: PathBuf, db: PathBuf) -> Result<(), String> {
    let extractor = extract::Extractor::load(&path)?;
    let body = pull(&extractor.url).await?;
    let rows = extract::apply(&extractor, &body)?;
    let conn = extract::open_db(&db)?;
    let fetched_at = fetched_now();
    for row in &rows {
        extract::upsert(&conn, row, &fetched_at)?;
        println!("{}\t{}\t{}\t{}\t{}", row.source, row.asset, row.place, row.date, row.title);
    }
    eprintln!("{} rows stored in {}", rows.len(), db.display());
    Ok(())
}

fn ask_intel(place: String, db: PathBuf) -> Result<(), String> {
    let conn = extract::open_db(&db)?;
    let rows = extract::ask_place(&conn, &place)?;
    for row in &rows {
        println!("{}\t{}\t{}\t{}\t{}", row.source, row.asset, row.place, row.date, row.title);
    }
    eprintln!("{} rows for {place}", rows.len());
    Ok(())
}

async fn pull(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("gem-probe/0.1 (public page read; +https://github.com/tsinghkothari-droid/gem-probe)")
        .timeout(std::time::Duration::from_secs(40))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status} {url}"));
    }
    Ok(body)
}

fn fetched_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

async fn watch(
    profile: Profile,
    accounts: &[Account],
    within: u32,
    out: Option<PathBuf>,
    state: Option<PathBuf>,
    organisation_options: organisation::Options,
) -> Result<(), String> {
    eprintln!("{}: {}", profile.company, profile.does);
    let today = row::today_ymd();
    let mut seen = load_seen(state.as_ref())?;
    let known: HashSet<String> = seen.bids.iter().cloned().collect();
    organisation::select(&organisation_options)?;
    let client = Gem::connect().await;
    let mut rows = Vec::new();
    let mut this_run = HashSet::new();
    let mut phrases = Vec::new();

    match client {
        Ok(client) => {
            for phrase in &profile.phrases {
                let status = status_code(phrase.status);
                let limit = match phrase.status {
                    BidStatus::Awarded => phrase.pages.min(2),
                    BidStatus::Ongoing => phrase.pages,
                };
                let label = match phrase.status {
                    BidStatus::Awarded => format!("{} (awarded)", phrase.query),
                    BidStatus::Ongoing => phrase.query.clone(),
                };
                let first = match client.search(&phrase.query, 1, status).await {
                    Ok(page) => page,
                    Err(err) if err.contains("no results") => {
                        eprintln!("no results  {label}");
                        phrases.push(PhraseStat {
                            query: label,
                            in_text: 0,
                            in_index: 0,
                            missing: true,
                            error: None,
                        });
                        continue;
                    }
                    Err(err) => {
                        eprintln!("skip {label}: {err}");
                        phrases.push(PhraseStat {
                            query: label,
                            in_text: 0,
                            in_index: 0,
                            missing: false,
                            error: Some(err),
                        });
                        continue;
                    }
                };
                let pages = limit.min(first.num_found.div_ceil(10).max(1) as u32);
                let mut text_ids = HashSet::new();
                for page_no in 1..=pages {
                    let page = if page_no == 1 {
                        first.clone()
                    } else {
                        match client.search(&phrase.query, page_no, status).await {
                            Ok(page) => page,
                            Err(err) => {
                                eprintln!("skip {label} page {page_no}: {err}");
                                break;
                            }
                        }
                    };
                    for doc in &page.docs {
                        let bid = bid_from_doc(doc);
                        if bid.number.is_empty() {
                            continue;
                        }
                        if term_hit(&bid.text, &phrase.query) {
                            text_ids.insert(bid.number.clone());
                        }
                        let key = format!("GeM:{}", bid.number);
                        if known.contains(&key) || known.contains(&bid.number) || this_run.contains(&key) {
                            continue;
                        }
                        let Some(verdict) = judge(&bid.text, &profile) else { continue };
                        this_run.insert(key);
                        rows.push(row_from_bid(&bid, verdict));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
                let in_text = text_ids.len() as u64;
                eprintln!("{in_text} in text / {} in index  {label}", first.num_found);
                phrases.push(PhraseStat {
                    query: label,
                    in_text,
                    in_index: first.num_found,
                    missing: false,
                    error: None,
                });
            }

            for row in rows.iter_mut() {
                if row.coverage != Coverage::Bill || row.matched.is_none() {
                    continue;
                }
                if !matches!(row::when(&today, &row.end, within), When::Inside | When::Undated) {
                    continue;
                }
                let Some(id) = row.doc_id else { continue };
                match client.document_text(id).await {
                    Ok(text) => apply_document(row, &text, &profile),
                    Err(err) => eprintln!("skip document {id}: {err}"),
                }
            }
        }
        Err(error) => {
            eprintln!("GeM unavailable: {error}; continuing other feeds");
            phrases.push(PhraseStat { query: "GeM connection".into(), in_text: 0, in_index: 0,
                                     missing: false, error: Some(error) });
        }
    }
    let (organisations, lists) = tokio::join!(organisation::fetch(organisation_options), cppp::list_feeds());
    let mut feeds = Vec::new();
    match organisations {
        Ok(reports) => {
            let mut tender_ids = HashSet::new();
            for report in reports {
                let count = report.tenders.len();
                let detail = report.coverage();
                let error = report.failure();
                eprintln!("{}: {count} rows; {detail}", report.host);
                for notice in report.tenders {
                    if !tender_ids.insert(notice.id.clone()) { continue; }
                    let text = format!("{} {} {}", notice.title, notice.reference, notice.buyer_chain);
                    push_title_row(&mut rows, &mut this_run, &known, &profile,
                        if report.kind == "bihar" { "Bihar EPS".into() } else { "GePNIC".into() }, notice.id, notice.buyer_chain, notice.title, text,
                        notice.closing, notice.source_url);
                }
                feeds.push(FeedStat { name: report.host, rows: count, error, detail: Some(detail) });
            }
        }
        Err(error) => feeds.push(FeedStat { name: "Public tender collector".into(), rows: 0,
                                          error: Some(error), detail: None }),
    }
    for (name, result) in lists {
        match result {
            Ok(notices) => {
                let count = notices.len();
                eprintln!("{count:4} rows, title only  {name}");
                for notice in notices {
                    let text = notice.text();
                    push_title_row(
                        &mut rows,
                        &mut this_run,
                        &known,
                        &profile,
                        notice.source,
                        notice.id,
                        notice.buyer,
                        notice.title,
                        text,
                        notice.closing,
                        notice.link,
                    );
                }
                feeds.push(FeedStat { name, rows: count, error: None, detail: None });
            }
            Err(err) => {
                eprintln!("skip {name}: {err}");
                feeds.push(FeedStat { name, rows: 0, error: Some(err), detail: None });
            }
        }
    }

    for row in &mut rows {
        row.account = accounts::match_buyer(&row.buyer, accounts);
    }
    rows.sort_by(|a, b| a.end.cmp(&b.end).then(a.id.cmp(&b.id)));
    let reported = rows.iter().filter(|row| include_in_report(row, &today, within)).cloned().collect::<Vec<_>>();
    write_report(&profile.company, &today, within, &rows, &phrases, &feeds, out.as_ref())?;
    if let Some(path) = state {
        for row in &reported {
            seen.bids.push(row.key());
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let file = File::create(&path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(file, &seen).map_err(|e| e.to_string())?;
        eprintln!("state {}", path.display());
    }
    eprintln!("{} rows for {}", reported.len(), profile.company);
    Ok(())
}

fn status_code(status: BidStatus) -> &'static str {
    match status {
        BidStatus::Ongoing => "ongoing_bids",
        BidStatus::Awarded => "bid_awarded",
    }
}

fn row_from_bid(bid: &gem::Bid, verdict: Verdict) -> Row {
    let (matched, dropped) = match verdict {
        Verdict::Keep(term) => (Some(term), None),
        Verdict::Drop(term) => (None, Some(term)),
    };
    Row {
        source: "GeM".into(),
        id: bid.number.clone(),
        buyer: buyer_fields(&bid.ministry, &bid.department),
        title: bid.title.clone(),
        text: bid.text.clone(),
        end: bid.end.clone(),
        link: bid.pdf_url(),
        matched,
        dropped,
        coverage: Coverage::Bill,
        account: None,
        doc_id: bid.id,
    }
}

fn apply_document(row: &mut Row, text: &str, profile: &Profile) {
    let combined = format!("{} {text}", row.text);
    match judge(&combined, profile) {
        Some(Verdict::Drop(term)) => {
            row.matched = None;
            row.dropped = Some(term);
            row.text = combined;
        }
        Some(Verdict::Keep(term)) => {
            row.matched = Some(term);
            row.text = combined;
        }
        None => {}
    }
}

struct TitleNotice {
    source: String,
    id: String,
    buyer: String,
    title: String,
    text: String,
    end: String,
    link: String,
}

fn push_title_row(
    rows: &mut Vec<Row>,
    this_run: &mut HashSet<String>,
    known: &HashSet<String>,
    profile: &Profile,
    source: String,
    id: String,
    buyer: String,
    title: String,
    text: String,
    end: String,
    link: String,
) {
    let notice = TitleNotice { source, id, buyer, title, text, end, link };
    if notice.id.is_empty() {
        return;
    }
    let key = format!("{}:{}", notice.source, notice.id);
    if known.contains(&key) || known.contains(&notice.id) || !this_run.insert(key) {
        return;
    }
    let Some(verdict) = judge(&notice.text, profile) else {
        this_run.remove(&format!("{}:{}", notice.source, notice.id));
        return;
    };
    let (matched, dropped) = match verdict {
        Verdict::Keep(term) => (Some(term), None),
        Verdict::Drop(term) => (None, Some(term)),
    };
    rows.push(Row {
        source: notice.source,
        id: notice.id,
        buyer: notice.buyer,
        title: notice.title,
        text: notice.text,
        end: notice.end,
        link: notice.link,
        matched,
        dropped,
        coverage: Coverage::Title,
        account: None,
        doc_id: None,
    });
}

fn buyer_fields(ministry: &str, department: &str) -> String {
    [ministry, department].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
}

fn include_in_report(row: &Row, today: &str, within: u32) -> bool {
    if row.dropped.is_some() {
        return true;
    }
    if row.matched.is_none() {
        return false;
    }
    if row.coverage == Coverage::Title {
        return true;
    }
    matches!(row::when(today, &row.end, within), When::Inside | When::Undated)
}

async fn sample() -> Result<(), String> {
    let started = std::time::Instant::now();
    let (gem, lists, boards) = tokio::join!(
        async {
            match Gem::connect().await {
                Ok(client) => match client.search("plastering", 1, "ongoing_bids").await {
                    Ok(page) => Ok(page.docs.len()),
                    Err(err) => Err(err),
                },
                Err(err) => Err(err),
            }
        },
        cppp::probe(),
        gepnic::probe(),
    );
    report("GeM BidPlus", gem);
    for (name, result) in lists {
        report(&name, result);
    }
    for (name, result) in boards {
        report(&name, result);
    }
    eprintln!("sample finished in {:.1}s", started.elapsed().as_secs_f32());
    Ok(())
}

fn report(name: &str, result: Result<usize, String>) {
    match result {
        Ok(0) => println!("empty  {name}"),
        Ok(rows) => println!("ok     {rows:>4}  {name}"),
        Err(err) => println!("fail        {name}  {err}"),
    }
}

async fn boards(out: Option<PathBuf>) -> Result<(), String> {
    let notices = gepnic::fetch_open_boards().await;
    let mut writer = writer(out.as_ref())?;
    writeln!(writer, "source,bid,title,reference,organisation,closing,list").map_err(|e| e.to_string())?;
    for notice in &notices {
        writeln!(
            writer,
            "{},{},{},{},{},{},{}",
            csv(&notice.source),
            csv(&notice.id),
            csv(&notice.title),
            csv(&notice.reference),
            csv(&notice.organisation),
            csv(&notice.closing),
            csv(&notice.list_url),
        )
        .map_err(|e| e.to_string())?;
    }
    eprintln!("{} rows written{}", notices.len(), dest(out.as_ref()));
    Ok(())
}

async fn search_once(query: String, kind: Kind, pages: u32, out: Option<PathBuf>) -> Result<(), String> {
    let client = Gem::connect().await?;
    let first = client.search(&query, 1, "ongoing_bids").await?;
    let pages = pages.min(first.num_found.div_ceil(10).max(1) as u32);
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    for page_no in 1..=pages {
        let page = if page_no == 1 {
            first.clone()
        } else {
            client.search(&query, page_no, "ongoing_bids").await?
        };
        for doc in &page.docs {
            let bid = bid_from_doc(doc);
            if bid.number.is_empty() || !seen.insert(bid.number.clone()) {
                continue;
            }
            let Some(why) = classify(kind, &bid.text) else { continue };
            hits.push((why, bid));
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    println!("{:>4} kept / {:5}  {query}", hits.len(), first.num_found);
    let mut writer = writer(out.as_ref())?;
    writeln!(writer, "bid,why,query,ministry,department,title,work_text,bid_end,pdf").map_err(|e| e.to_string())?;
    for (why, bid) in &hits {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{}",
            csv(&bid.number),
            csv(why.label()),
            csv(&query),
            csv(&bid.ministry),
            csv(&bid.department),
            csv(&bid.title),
            csv(&snippet(&bid.text)),
            csv(&bid.end),
            csv(&bid.pdf_url()),
        )
        .map_err(|e| e.to_string())?;
    }
    eprintln!("{} bids written{}", hits.len(), dest(out.as_ref()));
    Ok(())
}

fn write_report(
    company: &str,
    today: &str,
    within: u32,
    rows: &[Row],
    phrases: &[PhraseStat],
    feeds: &[FeedStat],
    out: Option<&PathBuf>,
) -> Result<(), String> {
    let csv_out = out.map(|path| path.extension().and_then(|ext| ext.to_str()) == Some("csv")).unwrap_or(false);
    if csv_out {
        let mut writer = writer(out)?;
        writeln!(writer, "source,id,buyer,matched,dropped,coverage,account,end,link,text").map_err(|e| e.to_string())?;
        for row in rows.iter().filter(|row| include_in_report(row, today, within)) {
            let coverage = match row.coverage {
                Coverage::Bill => "bill",
                Coverage::Title => "title",
            };
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{},{},{}",
                csv(&row.source),
                csv(&row.id),
                csv(&row.buyer),
                csv(row.matched.as_deref().unwrap_or("")),
                csv(row.dropped.as_deref().unwrap_or("")),
                coverage,
                csv(row.account.as_deref().unwrap_or("")),
                csv(&row.end),
                csv(&row.link),
                csv(&snippet_around(&row.text, row.matched.as_deref().or(row.dropped.as_deref()).unwrap_or(""))),
            )
            .map_err(|e| e.to_string())?;
        }
    } else {
        let page = row::render(company, today, within, rows, phrases, feeds);
        let mut writer = writer(out)?;
        write!(writer, "{page}").map_err(|e| e.to_string())?;
    }
    eprintln!("report written{}", dest(out));
    Ok(())
}

fn load_seen(path: Option<&PathBuf>) -> Result<Seen, String> {
    let Some(path) = path else { return Ok(Seen::default()) };
    if !path.exists() {
        return Ok(Seen::default());
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    serde_json::from_reader(file).map_err(|e| format!("state {}: {e}", path.display()))
}

fn writer(out: Option<&PathBuf>) -> Result<Box<dyn Write>, String> {
    match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            Ok(Box::new(File::create(path).map_err(|e| e.to_string())?))
        }
        None => Ok(Box::new(io::stdout())),
    }
}

fn dest(out: Option<&PathBuf>) -> String {
    out.map(|p| format!(" to {}", p.display())).unwrap_or_default()
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
