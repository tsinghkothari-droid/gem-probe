# Daily tender workflow

Use `run` for a company profile. `watch` uses the same collection path.

```bash
cargo run --release -- run \
  --profile profiles/navoday-sciences.json \
  --within 14 \
  --out out/navoday.md \
  --state state/navoday.json
```

The run searches GeM, collects GePNIC, Bihar EPS, Karnataka, and Rajasthan SHPP listings, reads AP/Telangana homepage notices, and reads four CPPP lists. It reports feed errors and partial coverage. Title-only matches remain separate from matches in bill text.

Use `--portal HOST` to select public listing hosts. Use `--max-organisations N` for a bounded partial scan. These options do not limit GeM phrases or CPPP lists. The default scans all configured hosts and GePNIC organisations. Karnataka and Rajasthan use `--listing-pages`, which defaults to two per category. AP and Telangana have homepage-only coverage.

Use `organisations --out out/active.json` to export raw public tender records and coverage counts. Use `boards` for the older six-site by-date diagnostic. `fetch` and `ask` remain separate auction/database tools.

Start a new company with `prompt`, then save and review its profile JSON. Use buyer terms rather than brand names. `keep` and `drop` control matches. Account names match buyer fields.

The collector does not submit a captcha, an OTP, or a login. CPPP Results of Tenders, OTP-only services, and other unimplemented portals remain outside the active-list collection.

See [README.md](README.md) for commands and [integration-validation.md](integration-validation.md) for tested results.
