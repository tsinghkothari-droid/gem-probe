# gem-probe

Watch the public [GeM BidPlus](https://bidplus.gem.gov.in/all-bids) feed for the work a company sells.

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

`run` is the daily command. `watch` calls the same path.

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

`run` reads the same six pages. A title that contains a keep term is listed under title matches. It is not treated as bill text.

## CPPP lists

These four pages already print tender titles. The keyword box on the same site asks for an image code. The tool reads the tables and does not submit that code.

- State and UT notices
- Central notices, including CPWD and Military Engineer Services
- High-value central notices
- Notices closing today

`run` reads the newest rows on each list. Those rows are title only. A keep term in the title is listed under title matches.

## One search

```bash
cargo run -- search "plastering" --pages 2
cargo run -- search "residential quarter" --kind house --pages 3
```

`--kind` is `plaster`, `masonry`, or `house`.
