# Wisconsin Circuit Court Access (WCCA) Case Data

## Source

**Wisconsin Circuit Court Access (WCCA)**
https://wcca.wicourts.gov

## License

Public Record — Wisconsin Supreme Court Rule 72

## How This Data Was Produced

Unlike other benchmark datasets that are fetched from public APIs, WCCA case detail
requires solving an hCaptcha per request. The raw JSON was **manually captured**
from the browser and then processed with `process_wcca.py`.

### Manual Capture Steps

1. Open the case detail page in a browser:
   `https://wcca.wicourts.gov/caseDetail.html?caseNo=<CASE_NO>&countyNo=<COUNTY_NO>&mode=details`

2. Open browser DevTools (F12) → **Network** tab

3. Solve the hCaptcha; the browser will POST to:
   `https://wcca.wicourts.gov/caseDetail/<countyNo>/<caseNo>`

4. In the Network tab, find the POST request and copy the JSON response

5. Save as `wisconsin_circuit_court_<DA_CASE_NO>.json` in this directory

### Processing

```bash
# Install dependencies (minimal — mostly stdlib)
pip install polars

# Process all captured case files into benchmark format
python process_wcca.py

# Process specific case(s)
python process_wcca.py --cases 2015CO001313
```

The processing script:
- Strips internal UI fields (keys, boolean flags, empty arrays)
- Renames fields for clarity (`attys` → `attorneys`, `chargeModifiers` → `modifiers`)
- Sorts docket entries chronologically (oldest first)
- Strips HTML from executive summary
- Wraps cases in metadata envelope

### Current Cases

| DA Case No | Court Case No | County | Type | Caption |
|------------|---------------|--------|------|---------|
| 2015CO001313 | 2016CF000031 | Columbia | Criminal Felony | State of Wisconsin vs. Luis A Ramirez |

### Output Schema

```json
{
  "source": "Wisconsin Circuit Court Access (WCCA)",
  "license": "Public Record — Wisconsin Supreme Court Rule 72",
  "case_count": 1,
  "cases": [
    {
      "county": "Columbia",
      "caseNo": "2016CF000031",
      "caseType": "CF",
      "classType": "Criminal",
      "caption": "State of Wisconsin vs. Luis A Ramirez  #244212",
      "status": "Closed",
      "filingDate": "2016-02-01",
      "defendant": {
        "name": "...",
        "dob": "...",
        "sex": "...",
        "race": "...",
        "attorneys": [ { "name", "entered", "withdrawn" } ],
        "aliases": [ { "name", "descr", "dob" } ]
      },
      "charges": [
        {
          "chargeNo": 1,
          "descr": "Battery by Prisoners",
          "severity": "Felony H",
          "statuteCite": "940.20(1)",
          "pleaDescr": "Not Guilty",
          "dispoDesc": "Found Guilty at Jury Trial",
          "offenseDate": "2015-05-05",
          "pleaDate": "2019-12-04",
          "modifiers": [ { "descr", "statuteCite" } ],
          "judgments": [
            {
              "action": "Found Guilty at Jury Trial",
              "sentDate": "2020-02-24",
              "supervisions": [
                {
                  "descr": "State Prison",
                  "time": "12 Years",
                  "notes": "...",
                  "conditions": [ { "descr", "notes" } ]
                }
              ]
            }
          ]
        }
      ],
      "chargeHistory": [ ... ],
      "docketEntries": [
        {
          "date": "2016-02-01",
          "descr": "Criminal complaint",
          "addlTxt": "...",
          "amount": null,
          "ctofc": "..."
        }
      ],
      "receivables": [ { "amountDue", "paid", "balanceDue", ... } ]
    }
  ]
}
```

This nested structure produces multiple TeaLeaf schemas: supervision with conditions,
judgment, charge with modifiers, defendant with attorneys and aliases, docket entry,
and receivable structs.
