# One gem-probe

There is one git branch, `main`. The working tree is seven commands that do not share one result. This file is the plan to make them one run.

## What is in the folder now

| Command | What it does | Keep in the one run? |
| --- | --- | --- |
| `watch` | GeM phrases, then six by-date boards, then four CPPP title lists | Yes. This becomes `run`. |
| `navoday` | Same watch, with the plaster profile baked in | No. It is `run --profile profiles/navoday-sciences.json`. |
| `search` | GeM search with plaster, masonry, or house rules in `filter.rs` | No. Those rules belong in a profile. |
| `prompt` | Prints the prompt that writes a profile. It does not search. | Yes, once, when a company is new. |
| `boards` | Dumps the six open by-date tables | Fold into `run`. Do not use it as the daily command. |
| `sample` | Prints whether each public page returned rows | Fold in as the last lines of the run. |
| `fetch` / `ask` | Reads auction pages into `intel.db` and asks by place | No. C1 India, MSTC, and JDA are a second product. They do not match a company profile. |

`profiles/navoday-sciences.json` and `profiles/varonis.json` are the company files. `extractors/*.json` stay out of the tender run.

## How to run it

**Today**, for a company that already has a profile:

```bash
cargo run --release -- watch --profile profiles/varonis.json --out /tmp/varonis-watch.csv
```

That is the only daily command. Do not start with `search`, `boards`, or `fetch`.

**Target**, after the gaps below are built:

```bash
cargo run --release -- run \
  --profile profiles/varonis.json \
  --accounts accounts/varonis-gcv.json \
  --within 14 \
  --out out/varonis.md \
  --state state/varonis.json
```

One profile in. One markdown file out. The file has four parts, in this order:

1. **Act this week.** Bids whose text contains a keep term and whose end date is inside `--within` days. Each line has the bid number, buyer, close date, the keep term, and the PDF link.
2. **On your account list.** The same bids whose buyer matches a name in `--accounts`. If none match, the file says so in one line.
3. **Lookalikes dropped.** Rows the index returned that failed the drop list, with the drop term. Example: CCTV “DLP support”.
4. **What the feeds did.** For each phrase: rows in the index, rows whose text contained the phrase. For boards and CPPP: row count, then “title only”. For a blocked page: “blocked”, not a zero.

`--state` stores bid numbers already printed. The next run prints only new bids, then repeats anything that still closes inside the window.

A new company starts one step earlier:

```bash
cargo run --release -- prompt \
  --company "Varonis" \
  --does "Data classification, permissions, and data loss prevention for file shares and Microsoft 365"
```

Save that JSON as `profiles/varonis.json`. Brand names are not phrases. A buyer writes “data loss prevention”, not “Varonis”.

## Top 10 gaps

These are the holes that made the Varonis and Navoday runs hard to use. They are in build order.

1. **Say when the GeM index lied.** “data loss prevention” returns 285 bids. The bill text on those pages does not contain the phrase. The run must print `0 in text / 285 in index`, not `0 kept / 285`.

2. **Read the PDF for a kept bid.** GeM gives about 1,000 characters. That text cannot tell file-share DLP from “Trellix endpoint protection and dlp”. Download `showbidDocument/{id}` for kept bids only, and match the keep list against that text.

3. **Match buyers from a file.** The GCV register has 660 Rajasthan and Gujarat accounts. Searching the legal name as GeM full text does not filter. “Department of Science Technology” returned the whole index. Match the buyer fields `ba_official_details_minName` and `ba_official_details_deptName` against the account file.

4. **One output page.** Stop writing a wide CSV as the main result. Write `out/<company>.md` in the four parts above. Sort by close date.

5. **Match whole terms.** `dlp` inside “DLP support for CCTV” is not data loss prevention. Match on word boundaries. Put the dropped row in “Lookalikes dropped” so a person can see the miss.

6. **Search past awards.** The public feed also has `bid_awarded` (about 5.7 million). The product was meant to look back at who already bought this work. Add `"status": "ongoing"` or `"awarded"` on each phrase. Default the daily run to ongoing. Use awarded when the question is “who already bought this”.

7. **Keep a closing window.** Ongoing GeM has about 46,000 bids. A watch without `--within 14` is a dump. Default the window to 14 days.

8. **Use one row shape.** A GeM bid, a board row, and a CPPP row are three structs today. One record: source, id, buyer, title, text, end, link, matched term. Then `--state` and the markdown file work for every source.

9. **Label title-only feeds.** The six boards and the CPPP lists do not carry bill text. A plaster or DLP term is rarely in the title. The run must not treat “0 kept” on those lists as “no work in that state”.

10. **Store the account list as data, not as a search.** `accounts/varonis-gcv.json` should hold legal name, short name, state, and tier from the GCV register. The run joins buyers to that file. It does not paste 660 names into GeM search.

## What we will not add

- No captcha, OTP, or login on GePNIC, nprocure, IREPS, or the CPPP keyword box.
- No Tender Tiger.
- No guessed emails. A PDF email is kept only when it is printed on the page.
- Auction extractors stay a separate command until a company profile needs that source.

## Build next

- [x] Add `run` as the name for today’s `watch`, and point the README at that one command.
- [x] Change the phrase line to `in text / in index`.
- [x] Add `--within` and the four-part markdown file.
- [x] Add `--accounts` and match buyer fields, not full text.
- [x] Download the PDF only for rows that already matched.

After those five, the Varonis command in “How to run it” is the daily habit.
