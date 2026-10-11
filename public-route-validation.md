# Simple public tender routes — 11 October 2026

The HTTP configuration error is fixed. Goa and Madhya Pradesh now use HTTPS. Both returned records with TLS verification enabled. An invalid URL produces a per-host configuration error rather than stopping the whole batch.

Four public adapters were added. They use Python's standard library and the existing curl client. No paid solver, CAPTCHA submission, browser automation, or login is required for these routes.

## Bounded all-host result

The configured batch completed for all 49 hosts. 48 returned records. Mizoram remained `challenge_required`. The run returned 3,429 source records before cross-host deduplication.

The run selected one GePNIC organisation per host and one API page per category. It did not establish complete portal coverage.

| Website | Records in bounded check | Verified scope |
|---|---:|---|
| Goa | 3 | One of 33 listed organisations; count matched |
| Madhya Pradesh | 16 | One of 97 listed organisations; count matched |
| Karnataka KPPP | 300 | 100 each from goods, works, and services; published OPEN tenders |
| Rajasthan SHPP | 146 | 46 SHPP records plus 100 SPPP records |
| Andhra Pradesh | 15 | Public homepage notices only |
| Telangana | 5 | Public homepage notices only |

Karnataka's category totals in this snapshot were 660 goods, 1,353 works, and 1,120 services. These filters exclude reserved and prequalification tenders. IDs include the category to avoid collisions between the three record sets.

The 46 SHPP records matched its advertised count. The SPPP endpoint reports counts that increase with the requested page. The collector therefore leaves the full SPPP total unknown and retains partial status. Repeated records are deduplicated.

AP and Telangana first return an automatic navigation form. Following it with the issued anonymous cookies and hidden fields returns public homepage notices. These may include corrigenda. Full-list requests failed: AP returned an empty response, and Telangana returned a session-termination page. The adapters use the working homepage route. They retain IDs and titles; buyer names, references, and dates stay empty where they were not extracted.

## What remains unresolved

Mizoram CAPTCHA, CPPP awards, and IREPS OTP access remain unresolved. nProcure and gepnic.gov.in were not re-tested in this change. AP and Telangana full-list access also remains unresolved.

## Validation

- 28 Rust unit tests passed; two existing live tests were excluded from the offline suite.
- 22 Python tests passed.
- Regression checks cover HTTPS configuration, per-host failure isolation, unknown totals, repeated pages, category-specific IDs, retained partial results, and homepage parsing.
- The bounded all-host command exited successfully and produced 49 separate reports.
- Existing unused-code compiler warnings remain.

## Commands

Run a bounded check of every configured website:

```bash
cargo run --release -- organisations --max-organisations 1 --listing-pages 1 \
  --portal-timeout 25 --out out/all-hosts.json
```

Run the company profile against the added websites:

```bash
cargo run --release -- run --profile profiles/navoday-sciences.json \
  --portal kppp.karnataka.gov.in --portal shpp.rajasthan.gov.in \
  --portal tender.apeprocurement.gov.in --portal tender.telangana.gov.in \
  --listing-pages 2 --out out/company.md
```

`--listing-pages` defaults to two API pages per category or source. `--max-organisations` applies only to GePNIC. Larger limits extend collection but do not by themselves establish full coverage.

## Evidence sources

The website's own public scripts supplied each route. The requests were then tested directly.

- [Goa organisation listings](https://eprocure.goa.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page)
- [Madhya Pradesh organisation listings](https://mptenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page)
- [Karnataka KPPP](https://kppp.karnataka.gov.in/): public `portal-service` category-search requests and `X-Total-Count` headers.
- [Rajasthan SHPP](https://shpp.rajasthan.gov.in/): public `publishedTenders` and `publishedTendersSppp` requests.
- [Andhra Pradesh](https://tender.apeprocurement.gov.in/): anonymous entry form and public `login.html` homepage.
- [Telangana](https://tender.telangana.gov.in/): anonymous entry form and public `login.html` homepage.

Repository evidence is saved in `public-route-validation.json`. Full raw tender exports are attached to the chat. Session cookies, form tokens, and downloaded application scripts are excluded from the committed evidence.

## Two-page follow-up

A second test of the default two-page limit returned 400 distinct Karnataka records: 200 goods and 200 services. Its works request returned HTTP 503. The error was recorded and the other records were retained. The earlier all-host run had successfully returned 100 works records. This shows that availability can vary between requests.

Rajasthan returned 189 distinct records: 46 SHPP and 143 SPPP. The two SPPP pages overlapped, so duplicate IDs were removed. Both website reports remain partial.
