# Public tender integration — 11 October 2026

The daily `run` command now includes public GePNIC organisation listings and Bihar EPS. No paid solver is required or configured.

## Live results

| Source | Records | Scope verified |
|---|---:|---|
| Bihar EPS | 847 | Full public active list; advertised total 847; department names included |
| Arunachal | 2 | Both listed organisations; both counts matched |
| BEL | 29 | Bharat Electronics Limited only; advertised count matched |
| HSL | 12 | Hindustan Shipyard Limited only; advertised count matched |
| MDL | 4 | Mazagon Dock Shipbuilders Limited only; advertised count matched |
| GSL | 1 | Goa Shipyard Limited only; advertised count matched |

The company sites share a directory. Each adapter now selects the intended buyer. The daily run deduplicates GePNIC IDs across hosts.

GeM completed the existing 12-phrase Navoday run. A separate integrated Bihar run returned one matching construction tender. Its GeM query returned two index results and zero phrase matches in the text. The report kept this distinction.

The CPPP pagination test returned nine records on its first page and 15 distinct records across two pages. The code now follows the page's encoded next link. The final daily test returned 19 state, 15 central, 18 high-value, and five closing-today records before cross-feed matching and deduplication.

Counts describe the tested snapshots. They can change as tenders open or close. This integration test did not scan every buyer on all 44 GePNIC hosts. The earlier bounded survey returned records from 43 hosts.

## Remaining gaps

- Mizoram rejected both automatic local OCR attempts, including the case-aware retry. It remains `challenge_required`. A separate assisted submission returned an empty table, which did not establish a verified zero.
- CPPP Results of Tenders still requires its search CAPTCHA. Active CPPP lists do not establish awards coverage.
- IREPS OTP access, JavaScript-only portals, and timed-out hosts from the earlier survey remain outside this implementation.
- Listings provide title, reference, buyer, and date fields. They do not establish matches inside full tender documents.

The optional local fallback uses ddddocr, Pillow, and, when installed, Tesseract. It attempts at most twice per selected host. No paid service or account key is used. Blocked listings report an unknown advertised total rather than zero.

## Checks

- 27 Rust unit tests passed; two live tests were excluded from the offline suite.
- 13 Python tests passed.
- The focused live CPPP pagination test passed separately.
- A forced connection failure confirmed that GeM failure does not stop other feeds or report generation.
- The final daily Bihar test completed with its full 847-record count and one profile match.

The existing build reports unused-code warnings. The installed Rust toolchain lacks rustfmt, so formatting was reviewed manually.

## Run it

From the repository directory:

```bash
cargo run --release -- run --profile profiles/navoday-sciences.json \
  --portal eproc2.bihar.gov.in --portal arunachaltenders.gov.in \
  --out out/navoday.md
```

Omit `--portal` to scan all configured public listing hosts. The Rust default scans all listed organisations. A full scan can take several minutes. `--max-organisations N` produces a bounded partial scan. These options do not limit GeM phrases or CPPP lists.

```bash
cargo run --release -- organisations --portal eproc2.bihar.gov.in \
  --out out/bihar-active.json
```

See `integration-validation.json` in the repository for compact live evidence. Full records and daily reports are also saved with this chat.

## Primary sources used

- [Bihar public tender listing](https://eproc2.bihar.gov.in/EPSV2Web/openarea/tenderListingPage.action): anonymous public listing and statistics requests.
- [CPPP active central tenders](https://eprocure.gov.in/cppp/latestactivetendersnew/cpppdata): real pagination links and title rows.
- [CPPP Results of Tenders](https://eprocure.gov.in/cppp/resultoftendersnew/cpppdata): the separate CAPTCHA search form.
- [GeM BidPlus](https://bidplus.gem.gov.in/all-bids): live phrase searches.
- Each host URL and collection timestamp appears in the JSON evidence.
