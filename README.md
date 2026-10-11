# gem-probe

Watch public GeM, GePNIC, Bihar EPS, Karnataka KPPP, Rajasthan SHPP, AP, Telangana, and CPPP tender feeds for the work a company sells.

An agent looks at the company first: products, where they are used, and the words a tender bill would use. That becomes a profile. The watch searches those phrases and keeps a bid only when the returned bill text contains one of the company's keep terms. Re-running the same command with `--state` reports only bids that were not in the last run.

The feed returns a ministry, a department, a title, and about a thousand characters of the bill. It does not return an officer email. The bid PDF link is in the output.

## Write the profile

Start with the system prompt. It reads what the company sells and which services it can perform. It writes the buyer’s search words, a keep list, and a drop list for lookalike terms. It does not search.

```bash
cargo run -- prompt --company "Navoday Sciences" --does "Curing-reducing admixture for cement plaster, premixed plaster, and wall putty"
```

Save the JSON it asks for, then run the watch. [profiles/navoday-sciences.json](profiles/navoday-sciences.json) is that prompt’s output for one firm.

## Fetch a site

An extractor is one JSON file. It names a public URL and the fields to lift. The runner stores the rows in `intel.db`. A later question reads that database. The fetcher does not know the company.

```bash
cargo run -- fetch --extractor extractors/c1india.json
cargo run -- fetch --extractor extractors/mstc.json
cargo run -- fetch --extractor extractors/jda-rajasthan.json
cargo run -- ask --place Chennai
```

If the page asks for a captcha, an OTP, or a password and no row is found, the runner stores one row titled `blocked`.

## Watch a company

`run` is the daily command. `watch` calls the same path. Install Python 3 and curl alongside the Rust binary.

The run searches GeM phrases, collects listings from 49 configured hosts, and reads four CPPP lists. The hosts include 44 GePNIC portals, Bihar EPS, Karnataka KPPP, Rajasthan SHPP, AP, and Telangana. It scans every listed GePNIC organisation by default. Other coverage limits appear below. Large portals can take several minutes. A GeM connection failure appears in the report while other feeds continue.

```bash
cargo run --release -- run \
  --profile profiles/varonis.json \
  --accounts accounts/varonis-gcv.json \
  --within 14 \
  --out out/varonis.md \
  --state state/varonis.json
```

A `.csv` path writes a table. Any other path writes four parts: bids closing inside `--within` days, buyers on the account file, lookalikes, and what each feed returned. A phrase line looks like `0 in text / 285 in index`. A board or CPPP line says `title only`.

```bash
cargo run --release -- run --profile profiles/navoday-sciences.json --out out/navoday.md --state state/navoday.json
cargo run --release -- navoday --out out/navoday.md
cargo run --release -- watch --company "Northwind Pumps" --does "submersible pump, dewatering" --out out/pumps.md
```

Set `"status": "awarded"` on a phrase to search past awards. Those phrases stop after 2 pages.

A profile is JSON:

```json
{
  "company": "Navoday Sciences",
  "does": "Curing-reducing admixture for cement plaster, plus premixed plaster and wall putty.",
  "keep": ["cement plaster", "plastering", "wall putty"],
  "drop": ["plaster of paris bandage"],
  "phrases": [{"query": "12 mm cement plaster", "pages": 4}]
}
```

`keep` is what must appear in the bill. `phrases` are the searches. `drop` throws out false matches. Brand names are not search terms. Tender bills say "12 mm cement plaster", not the product name.

## Sample every connection

```bash
cargo run -- sample
```

Each line is `ok` with a row count, `empty` when the page opened and had no tender rows, or `fail` with the error.

## Six open state boards

These GePNIC sites print the day's tenders on the by-date page. The search box on the same page asks for a captcha. The tool reads the table and does not submit the captcha.

Rajasthan, Himachal Pradesh, Defence, Haryana, Coal India, and Jammu and Kashmir.

```bash
cargo run -- boards --out boards.csv
```

`boards` keeps this six-page diagnostic. The daily `run` command uses the broader organisation listings below. A title match is not treated as a match in bill text.

## CPPP lists

These four pages already print tender titles. The keyword box on the same site asks for an image code. The tool reads the tables and does not submit that code.

- State and UT notices
- Central notices, including CPWD and Military Engineer Services
- High-value central notices
- Notices closing today

`run` reads up to two pages per list, follows the actual encoded next-page links, and removes duplicate tender IDs. Those rows are title only. A keep term in the title is listed under title matches. CPPP Results of Tenders remains CAPTCHA-protected and is not covered by these active lists.

## One search

```bash
cargo run -- search "plastering" --pages 2
cargo run -- search "residential quarter" --kind house --pages 3
```

`--kind` is `plaster`, `masonry`, or `house`.

## Active tender collection

`run` and `watch` embed `organisation_tenders.py` and `gepnic-portals.json`. They preserve cookies per host and collect public active listings. The organisation directory works on many GePNIC hosts whose keyword search requires a CAPTCHA. Bihar uses an anonymous token issued by its public listing page.

Select hosts for a shorter run:

```bash
cargo run --release -- run --profile profiles/navoday-sciences.json \
  --portal arunachaltenders.gov.in --portal eproc2.bihar.gov.in \
  --out out/navoday.md
```

Export all collected records and coverage counts without profile filtering:

```bash
cargo run --release -- organisations \
  --portal eprocurebel.co.in --portal eproc2.bihar.gov.in \
  --out out/active-tenders.json
```

Omit `--portal` to select all configured hosts. Repeat it to select several. `--max-organisations 0` scans every matching organisation and is the Rust default. Set a positive limit for a partial scan. `--portal-timeout 90` sets the timeout for each request. `--max-organisations` applies only to GePNIC. `--portal-timeout` applies to each collector request. Bihar returns its public list in one response.

BEL, HSL, MDL, and GSL domains share a directory. Their configuration selects the named company, so an unrelated BEML listing does not count as that company's result. The daily run removes duplicate GePNIC tender IDs across hosts.

Each JSON report includes advertised and parsed counts, scan errors, and completeness flags. A complete selected-buyer listing does not imply complete coverage of the shared portal. Bihar compares its unique record count with the portal's active-tender count. Records remain available when a later request fails. Counts can change while a scan runs.

A partial scan with zero matches is not a portal-wide zero. All these listings contain titles and buyer names, not full tender documents or historical awards.

The standalone collector remains available:

```bash
python3 organisation_tenders.py \
  --url 'https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page' \
  --transport curl --max-orgs 0 --timeout 90 \
  --query road --out out/rajasthan-active.json
```

The standalone default is two organisations. Its local `--query` filter checks titles, references, and buyer chains. Normal daily profile matching checks these listing fields and labels the result as title-only coverage.

## Additional public routes

Goa and Madhya Pradesh now use verified HTTPS URLs. An invalid HTTP entry produces a `configuration_error` for that host without stopping valid entries.

Karnataka uses its public JSON listing for published OPEN tenders in goods, works, and services. `--listing-pages` defaults to two pages of 100 records per category. Increase it for broader coverage. A matching category total can establish complete selected coverage; reserved and prequalification tenders are outside this filter.

Rajasthan SHPP reads both its SHPP and SPPP published-tender feeds. The same page limit applies to each feed. SPPP's reported count increases with pagination, so it is not treated as a verified full total.

AP and Telangana follow the anonymous entry form and read public homepage notices. These may include corrigenda. Full-list requests remain unresolved. Homepage records have IDs and titles; buyer names, references, and dates remain empty where they were not extracted. Their reports always state partial coverage.

```bash
cargo run --release -- organisations --portal kppp.karnataka.gov.in \
  --portal shpp.rajasthan.gov.in --listing-pages 2 --out out/public-listings.json
```

A bounded check of every configured host:

```bash
cargo run --release -- organisations --max-organisations 1 --listing-pages 1 \
  --portal-timeout 25 --out out/all-hosts.json
```

No browser automation, paid solver, CAPTCHA submission, or login is required for these added routes. See [public-route validation](public-route-validation.md) for results and limits.

## Optional free CAPTCHA attempt

The default is `--captcha-solver none`. `--captcha-solver local` tries an embedded GePNIC image at most twice. It keeps the same session and refreshes hidden form fields after rejection. It uses `ddddocr` and Pillow in the selected Python 3 environment. If Tesseract is on `PATH`, it also checks character case. No paid service or API key is used.

```bash
python3 -m venv .venv
.venv/bin/python -m pip install ddddocr Pillow
PATH="$PWD/.venv/bin:$PATH" cargo run --release -- organisations \
  --portal mizoramtenders.gov.in --captcha-solver local --out out/mizoram.json
```

Live Mizoram tests on 11 October 2026 rejected the automatic predictions. The fallback is experimental, not a resolved connection. An empty response after submission remains unverified. The JSON keeps the challenge status and attempt outcomes without storing answers or session tokens.

## Validation

```bash
cargo test --offline
python3 -m unittest discover -s tests -p 'test_*.py'
```

A separate network test checks CPPP pagination:

```bash
cargo test cppp::tests::live_two_pages_have_distinct_tenders -- --ignored --exact --nocapture
```

See [integration validation](integration-validation.md) for live results and remaining gaps. Earlier bounded tests returned records from 43 of 44 GePNIC hosts; they did not scan every buyer on each host. See [the earlier findings](captcha-automation-findings.md) and [host evidence](organisation-route-summary.json).
