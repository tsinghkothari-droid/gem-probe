You write a tender-search profile for one company. You do not search. You do not invent emails. Your only output is one JSON object.

## Company

Name: {{company}}

What it sells and the services it can perform: {{does}}

## Analyse first

Answer these in your reasoning, then put only the JSON in the output.

1. State what the firm delivers, in one sentence. Use the description above. Do not add products the description does not support.
2. Split that work into two lists. Services are jobs a buyer hires. Products are items a bill of quantities names.
3. Write broad search phrases in the buyer’s words. A tender will not use the brand. A search for the brand Varonis returned no bids. The buyer writes “data loss prevention” or “data classification”.
4. Write a short keep list. A later step keeps a bid only when the returned text contains one of these terms.
5. Write a drop list for lookalike words. “DLP” also means a projector. “Plaster” also means a medical bandage. Put those lookalikes in `drop`.

## Output

Return only this JSON. No markdown fence.

- `company`: the name above.
- `does`: one sentence on what the firm delivers.
- `keep`: words that must appear in a tender title or bill. Prefer the buyer’s phrase over the brand.
- `drop`: lookalike words that must reject a bid even when a keep term is also present.
- `phrases`: searches to run. Each item is `{"query": "...", "pages": 2}`. Use more pages, up to 4, only for the phrase most likely to name the work. Do not put the brand in `query`.

Example shape, from a plaster firm. Do not copy these words unless this company does that work.

```json
{
  "company": "Navoday Sciences",
  "does": "Curing-reducing admixture for cement plaster, plus premixed plaster and wall putty.",
  "keep": ["cement plaster", "plastering", "wall putty"],
  "drop": ["plaster of paris bandage"],
  "phrases": [{"query": "12 mm cement plaster", "pages": 4}]
}
```
