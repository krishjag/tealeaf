# NIST National Vulnerability Database (NVD) CVE Data

## Source

**NIST National Vulnerability Database (NVD)**
https://nvd.nist.gov

## License

US Government Public Domain

## How This Data Was Produced

CVE vulnerability records are fetched from the NVD CVE API v2.0 with **minimal
processing** — the native NVD nesting is preserved. The API returns deeply nested
JSON with CVSS scoring metrics (v2.0, v3.0, v3.1), CWE weakness classifications,
affected product configurations using CPE (Common Platform Enumeration), and
external references with categorized tags.

### Steps

1. Query the NVD API at `https://services.nvd.nist.gov/rest/json/cves/2.0` with
   date range and optional keyword filters
2. Paginate through results (NVD rate limits: 5 requests per 30 seconds without API key)
3. Strip the API pagination envelope (`resultsPerPage`, `startIndex`, `totalResults`)
4. Recursively remove null values and empty collections
5. Sort chronologically by publication date
6. Wrap in metadata envelope with summary statistics

### Reproducing

```bash
# Install dependencies
pip install requests

# Fetch default dataset (Linux kernel CVEs, March 2024)
python process_nvd.py

# Custom date range and keyword
python process_nvd.py --start 2024-06-01 --end 2024-06-30 --keyword "apache"

# Larger dataset, no keyword filter
python process_nvd.py --start 2024-01-01 --end 2024-01-31 --keyword "" --limit 100
```

### Output Schema

The native NVD nesting is preserved — each CVE record retains the original
`metrics → cvssMetricV31 → cvssData` hierarchy (5 levels deep):

```json
{
  "source": "NIST National Vulnerability Database (NVD)",
  "license": "US Government Public Domain",
  "url": "https://nvd.nist.gov",
  "api": "NVD CVE API v2.0",
  "description": "40 CVE vulnerability records...",
  "query": { "keyword", "pubStartDate", "pubEndDate", "fetchedAt" },
  "summary": {
    "totalCves": 40,
    "severityDistribution": { "HIGH": 12, "MEDIUM": 28 },
    "uniqueCwes": 10,
    "topCwes": { "CWE-416": 8, "CWE-476": 5, ... },
    "uniqueVendors": 2,
    "topVendors": { "linux": 38, "debian": 2 }
  },
  "vulnerabilities": [
    {
      "cve": {
        "id": "CVE-2024-XXXXX",
        "sourceIdentifier": "cve@mitre.org",
        "published": "2024-03-01T...",
        "lastModified": "2024-04-01T...",
        "vulnStatus": "Analyzed",
        "descriptions": [
          { "lang": "en", "value": "In the Linux kernel..." }
        ],
        "metrics": {
          "cvssMetricV31": [
            {
              "source": "nvd@nist.gov",
              "type": "Primary",
              "cvssData": {
                "version": "3.1",
                "vectorString": "CVSS:3.1/AV:L/AC:L/PR:L/UI:N/S:U/C:H/I:H/A:H",
                "baseScore": 7.8,
                "baseSeverity": "HIGH",
                "attackVector": "LOCAL",
                "attackComplexity": "LOW",
                "privilegesRequired": "LOW",
                "userInteraction": "NONE",
                "scope": "UNCHANGED",
                "confidentialityImpact": "HIGH",
                "integrityImpact": "HIGH",
                "availabilityImpact": "HIGH"
              },
              "exploitabilityScore": 1.8,
              "impactScore": 5.9
            }
          ]
        },
        "weaknesses": [
          {
            "source": "nvd@nist.gov",
            "type": "Primary",
            "description": [ { "lang": "en", "value": "CWE-416" } ]
          }
        ],
        "configurations": [
          {
            "nodes": [
              {
                "operator": "OR",
                "negate": false,
                "cpeMatch": [
                  {
                    "vulnerable": true,
                    "criteria": "cpe:2.3:o:linux:linux_kernel:*:...",
                    "versionStartIncluding": "5.10",
                    "versionEndExcluding": "6.8",
                    "matchCriteriaId": "..."
                  }
                ]
              }
            ]
          }
        ],
        "references": [
          {
            "url": "https://git.kernel.org/...",
            "source": "cve@mitre.org",
            "tags": ["Patch"]
          }
        ]
      }
    }
  ]
}
```

This deeply nested structure (5 levels: vulnerabilities → cve → metrics →
cvssMetricV31 → cvssData) produces rich TeaLeaf schemas with significant
structural repetition ideal for `@struct`/`@table` compression.
