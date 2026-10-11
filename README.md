# gem-probe

Search the public [GeM BidPlus](https://bidplus.gem.gov.in/all-bids) feed for cement-plaster lines and house-building work.

The feed is `POST /all-bids-data`. Each bid returns a ministry, a department, a title, and about a thousand characters of the bill of quantities. An email is not in that JSON. The bid PDF link is included when GeM publishes one.

State procurement portals that use GePNIC are not searched. Their keyword form asks for a captcha.

## Run

```bash
cargo run -- navoday --out bids.csv
cargo run -- search "plastering" --pages 2
cargo run -- search "residential quarter" --kind house --pages 3
```

`--kind` is `plaster`, `masonry`, or `house`. A row is kept only when the returned bill text matches that kind. Medical plaster bandages, manpower notices, and carton-size matches are dropped.
