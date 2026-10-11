mod filter;
mod gem;

use clap::{Parser, Subcommand};
use filter::{classify, snippet, Kind, Why};
use gem::{bid_from_doc, Gem};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gem-probe", about = "Search public GeM BidPlus bids for plaster and house building")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run one search phrase and keep rows whose bill text matches.
    Search {
        query: String,
        #[arg(long, default_value_t = KindArg::Plaster)]
        kind: KindArg,
        #[arg(long, default_value_t = 3)]
        pages: u32,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Run the Navoday plaster and house-building phrase list.
    Navoday {
        #[arg(long)]
        out: Option<PathBuf>,
    },
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

struct Hit {
    query: String,
    why: Why,
    bid: gem::Bid,
}

struct Phrase {
    query: String,
    kind: Kind,
    pages: u32,
}

fn navoday_phrases() -> Vec<Phrase> {
    [
        ("12 mm cement plaster", Kind::Plaster, 8),
        ("15 mm cement plaster", Kind::Plaster, 6),
        ("20 mm cement plaster", Kind::Plaster, 4),
        ("internal plaster", Kind::Plaster, 4),
        ("external plaster", Kind::Plaster, 4),
        ("ceiling plaster", Kind::Plaster, 3),
        ("plastering", Kind::Plaster, 2),
        ("dismantling old plaster", Kind::Plaster, 3),
        ("cement plaster of mix", Kind::Plaster, 4),
        ("wall putty", Kind::Plaster, 3),
        ("brick masonry", Kind::Masonry, 4),
        ("half brick masonry", Kind::Masonry, 4),
        ("residential quarter", Kind::House, 3),
        ("staff quarter", Kind::House, 3),
        ("construction of barracks", Kind::House, 2),
        ("hostel building", Kind::House, 2),
    ]
    .into_iter()
    .map(|(query, kind, pages)| Phrase { query: query.to_string(), kind, pages })
    .collect()
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
    let phrases = match &cli.command {
        Command::Search { query, kind, pages, .. } => vec![Phrase {
            query: query.clone(),
            kind: (*kind).into(),
            pages: *pages,
        }],
        Command::Navoday { .. } => navoday_phrases(),
    };
    let out = match &cli.command {
        Command::Search { out, .. } | Command::Navoday { out } => out.clone(),
    };

    let client = Gem::connect().await?;
    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for phrase in &phrases {
        let first = match client.search(&phrase.query, 1).await {
            Ok(page) => page,
            Err(err) => {
                eprintln!("skip {}: {err}", phrase.query);
                continue;
            }
        };
        let pages = phrase.pages.min(first.num_found.div_ceil(10).max(1) as u32);
        let mut kept = 0u32;
        for page_no in 1..=pages {
            let page = if page_no == 1 {
                first.clone()
            } else {
                match client.search(&phrase.query, page_no).await {
                    Ok(page) => page,
                    Err(err) => {
                        eprintln!("skip {} page {page_no}: {err}", phrase.query);
                        break;
                    }
                }
            };
            for doc in &page.docs {
                let bid = bid_from_doc(doc);
                if bid.number.is_empty() || !seen.insert(bid.number.clone()) {
                    continue;
                }
                let Some(why) = classify(phrase.kind, &bid.text) else {
                    continue;
                };
                kept += 1;
                hits.push(Hit { query: phrase.query.to_string(), why, bid });
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        println!("{kept:4} kept / {:5}  {}", first.num_found, phrase.query);
    }

    hits.sort_by(|a, b| {
        why_rank(a.why)
            .cmp(&why_rank(b.why))
            .then(a.bid.department.cmp(&b.bid.department))
            .then(a.bid.number.cmp(&b.bid.number))
    });

    let mut writer: Box<dyn Write> = match &out {
        Some(path) => Box::new(File::create(path).map_err(|e| e.to_string())?),
        None => Box::new(io::stdout()),
    };
    writeln!(writer, "bid,why,query,ministry,department,title,work_text,bid_end,pdf").map_err(|e| e.to_string())?;
    for hit in &hits {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{}",
            csv(&hit.bid.number),
            csv(hit.why.label()),
            csv(&hit.query),
            csv(&hit.bid.ministry),
            csv(&hit.bid.department),
            csv(&hit.bid.title),
            csv(&snippet(&hit.bid.text)),
            csv(&hit.bid.end),
            csv(&hit.bid.pdf_url()),
        )
        .map_err(|e| e.to_string())?;
    }
    eprintln!("{} bids written{}", hits.len(), out.as_ref().map(|p| format!(" to {}", p.display())).unwrap_or_default());
    Ok(())
}

fn why_rank(why: Why) -> u8 {
    match why {
        Why::PlasterLine => 0,
        Why::MasonryLine => 1,
        Why::BuildingWorks => 2,
    }
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
