# gem-probe

Watch public Indian tender feeds for the work a company sells.

The tool reads published listings. It keeps a bid when the bill text contains a keep term from your profile. It does not log in, submit a captcha, or send an OTP.

## What you need

- Rust and Cargo
- Python 3
- curl

## Start here

1. Build the sample watch. This takes about one minute.

```bash
cargo run --release -- navoday --out out/navoday.md
```

2. Open `out/navoday.md`. The sample profile is [profiles/navoday-sciences.json](profiles/navoday-sciences.json).

`out/` is ignored by git. Do not commit a watch report.

## Write your own profile

`prompt` prints a system prompt. It does not search.

```bash
cargo run -- prompt \
  --company "Northwind Pumps" \
  --does "submersible pump, dewatering"
```

Save the JSON it describes, then run the watch:

```bash
cargo run --release -- run \
  --profile profiles/your-company.json \
  --within 14 \
  --out out/watch.md \
  --state state/watch.json
```

`--within` keeps bill matches that close inside that many days. `--state` reports only bids that were not in the last run.

A profile looks like this:

```json
{
  "company": "Navoday Sciences",
  "does": "Curing-reducing admixture for cement plaster, plus premixed plaster and wall putty.",
  "keep": ["cement plaster", "plastering", "wall putty"],
  "drop": ["plaster of paris bandage"],
  "phrases": [{"query": "12 mm cement plaster", "pages": 4}]
}
```

`phrases` are the searches. `keep` must appear in the bill. `drop` throws out false matches. Use the words a tender bill uses. A brand name is a poor search term.

Set `"status": "awarded"` on a phrase to search past awards. Those phrases stop after 2 pages.

A `.csv` output path writes a table. Any other path writes four parts: bids closing inside the window, buyers on an optional account file, lookalikes, and what each feed returned.

## Commands

| Command | What it does |
|---|---|
| `run` | Daily watch. `watch` is the same command. |
| `navoday` | The shipped sample profile. |
| `prompt` | Print the profile-writing prompt. |
| `search` | One GeM phrase. `--kind` is `plaster`, `masonry`, or `house`. |
| `boards` | Six GePNIC by-date pages that already print rows. |
| `organisations` | Public listings and coverage counts, with no profile filter. |
| `sample` | Open every feed once. Each line is `ok`, `empty`, or `fail`. |
| `fetch` | Run one extractor JSON and store rows in `intel.db`. |
| `ask` | Read stored rows for a place. |

```bash
cargo run -- search "plastering" --pages 2
cargo run -- boards --out boards.csv
cargo run -- sample
cargo run -- fetch --extractor extractors/c1india.json
cargo run -- ask --place Chennai
```

Ship extractors: `extractors/c1india.json`, `extractors/mstc.json`, `extractors/mstcauction.json`, `extractors/jda-rajasthan.json`.

## What a feed returns

A GeM row has a ministry, a department, a title, and about a thousand characters of the bill. The bid PDF link is in the output. The feed does not return an officer email.

A phrase line looks like `0 in text / 285 in index`. A board or CPPP line says `title only`. A title match is not a match in bill text.

## Where it looks

`run` searches GeM phrases, reads four CPPP title lists, and collects listings from the hosts in [gepnic-portals.json](gepnic-portals.json). Those hosts include GePNIC portals, Bihar EPS, Karnataka KPPP, Rajasthan SHPP, AP, and Telangana.

The four CPPP lists are state and UT notices, central notices, high-value central notices, and notices closing today. `run` reads up to two pages per list.

Limit a run to named hosts:

```bash
cargo run --release -- run \
  --profile profiles/navoday-sciences.json \
  --portal arunachaltenders.gov.in \
  --portal eproc2.bihar.gov.in \
  --out out/navoday.md
```

Export listings with no profile filter:

```bash
cargo run --release -- organisations \
  --portal eprocurebel.co.in \
  --out out/active-tenders.json
```

`--max-organisations 0` scans every matching GePNIC organisation. That is the Rust default. A positive number is a partial scan. `--portal-timeout 90` is the timeout for each request. `--listing-pages` defaults to 2 for Karnataka and Rajasthan feeds.

The same collector can run on its own. Its default is two organisations:

```bash
python3 organisation_tenders.py \
  --url 'https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page' \
  --transport curl --max-orgs 0 --timeout 90 \
  --query road --out out/rajasthan-active.json
```

## What it does not do

- It does not submit a captcha, an OTP, or a password.
- A directory that asks for a captcha is `challenge_required`. The advertised total stays unknown.
- If `fetch` hits a captcha, an OTP, or a password and finds no row, it stores one row titled `blocked`.
- It does not download full tender documents or historical awards.
- It does not read pages behind a login.

An optional `--accounts` file marks known buyers in the report. Keep that file outside the repo. `/accounts/` is in `.gitignore`.

## Coverage limits

- A partial scan with zero matches is not a portal-wide zero.
- BEL, HSL, MDL, and GSL share a directory. The config selects the named company.
- Bihar uses a public anonymous token. Its report compares unique records with the portal's active-tender count.
- Karnataka reads published OPEN tenders in goods, works, and services. Reserved and prequalification tenders stay outside that filter.
- Rajasthan SHPP reads the SHPP and SPPP published-tender feeds. SPPP's reported count grows with pagination, so it is not a verified full total.
- AP and Telangana read public homepage notices. Those reports always say the coverage is partial.
- Large portals can take several minutes. A GeM failure is listed in the report. Other feeds continue.

Live notes: [integration-validation.md](integration-validation.md), [public-route-validation.md](public-route-validation.md), [organisation-route-summary.json](organisation-route-summary.json).

## Tests

```bash
cargo test --offline
python3 -m unittest discover -s tests -p 'test_*.py'
```

One network test checks CPPP pagination:

```bash
cargo test cppp::tests::live_two_pages_have_distinct_tenders -- --ignored --exact --nocapture
```

## License

MIT.
