#!/usr/bin/env python3
"""Fetch CVE vulnerability data from the NIST National Vulnerability Database (NVD) API v2.0.

Downloads CVE records for a configurable time window with minimal processing —
the NVD response is already deeply nested and well-structured, so we preserve
the original schema (metrics → cvssData, configurations → nodes → cpeMatch, etc.)
and only strip empty fields and the API pagination envelope.

Usage:
    python process_nvd.py                                          # default: 2024-03-01 to 2024-03-31, keyword "linux kernel"
    python process_nvd.py --start 2024-06-01 --end 2024-06-30     # custom date range
    python process_nvd.py --keyword "apache" --limit 50            # custom keyword
    python process_nvd.py --start 2024-03-01 --end 2024-03-31 --keyword "linux kernel" --limit 40

Requirements:
    pip install requests
"""

import argparse
import json
import time
from pathlib import Path

import requests

DATA_DIR = Path(__file__).parent

NVD_API_BASE = "https://services.nvd.nist.gov/rest/json/cves/2.0"


def fetch_cves(
    start_date: str,
    end_date: str,
    keyword: str | None = None,
    limit: int = 50,
) -> list[dict]:
    """Fetch CVEs from NVD API v2.0 with pagination."""

    params = {
        "pubStartDate": f"{start_date}T00:00:00.000",
        "pubEndDate": f"{end_date}T23:59:59.999",
        "resultsPerPage": min(limit, 100),
        "startIndex": 0,
    }
    if keyword:
        params["keywordSearch"] = keyword

    headers = {
        "User-Agent": "TeaLeaf-Benchmark/1.0 benchmark@tealeaf-project.dev",
    }

    all_vulns = []
    total = None

    while True:
        print(f"  Fetching from index {params['startIndex']}...")
        resp = requests.get(NVD_API_BASE, params=params, headers=headers, timeout=60)
        resp.raise_for_status()
        data = resp.json()

        if total is None:
            total = data["totalResults"]
            print(f"  Total matching CVEs: {total}")

        vulnerabilities = data.get("vulnerabilities", [])
        if not vulnerabilities:
            break

        all_vulns.extend(vulnerabilities)

        if len(all_vulns) >= limit or len(all_vulns) >= total:
            break

        params["startIndex"] += len(vulnerabilities)

        # NVD rate limit: 5 requests per 30 seconds without API key
        print("  Waiting 6s for rate limit...")
        time.sleep(6)

    all_vulns = all_vulns[:limit]
    print(f"  Fetched {len(all_vulns)} CVEs")
    return all_vulns


def strip_empty(obj):
    """Recursively remove null values and empty lists/dicts from a JSON object."""
    if isinstance(obj, dict):
        return {
            k: strip_empty(v)
            for k, v in obj.items()
            if v is not None and v != [] and v != {}
        }
    elif isinstance(obj, list):
        return [strip_empty(item) for item in obj]
    return obj


def summarize(vulnerabilities: list[dict]) -> dict:
    """Build summary statistics from raw NVD vulnerability records."""
    severity_counts = {}
    all_cwes = {}
    all_vendors = {}

    for vuln in vulnerabilities:
        cve = vuln.get("cve", {})
        metrics = cve.get("metrics", {})

        # Severity from best available CVSS (v3.1 > v3.0 > v2)
        scored = False
        for key in ("cvssMetricV31", "cvssMetricV30"):
            if key in metrics:
                for entry in metrics[key]:
                    sev = entry.get("cvssData", {}).get("baseSeverity", "UNKNOWN")
                    severity_counts[sev] = severity_counts.get(sev, 0) + 1
                    scored = True
                    break
            if scored:
                break
        if not scored:
            severity_counts["UNSCORED"] = severity_counts.get("UNSCORED", 0) + 1

        # CWEs
        for w in cve.get("weaknesses", []):
            for desc in w.get("description", []):
                val = desc.get("value", "")
                if val.startswith("CWE-"):
                    all_cwes[val] = all_cwes.get(val, 0) + 1

        # Vendors from CPE
        for config in cve.get("configurations", []):
            for node in config.get("nodes", []):
                for match in node.get("cpeMatch", []):
                    if match.get("vulnerable"):
                        parts = match.get("criteria", "").split(":")
                        if len(parts) >= 5:
                            vendor = parts[3]
                            all_vendors[vendor] = all_vendors.get(vendor, 0) + 1

    return {
        "totalCves": len(vulnerabilities),
        "severityDistribution": dict(sorted(severity_counts.items())),
        "uniqueCwes": len(all_cwes),
        "topCwes": dict(sorted(all_cwes.items(), key=lambda x: -x[1])[:10]),
        "uniqueVendors": len(all_vendors),
        "topVendors": dict(sorted(all_vendors.items(), key=lambda x: -x[1])[:10]),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Fetch NVD CVE data for benchmark"
    )
    parser.add_argument(
        "--start", default="2024-03-01",
        help="Start date for CVE publication (YYYY-MM-DD)"
    )
    parser.add_argument(
        "--end", default="2024-03-31",
        help="End date for CVE publication (YYYY-MM-DD)"
    )
    parser.add_argument(
        "--keyword", default="linux kernel",
        help="Keyword search filter (default: 'linux kernel')"
    )
    parser.add_argument(
        "--limit", type=int, default=40,
        help="Maximum number of CVEs to fetch (default: 40)"
    )
    args = parser.parse_args()

    print(f"Fetching CVEs from NVD API v2.0...")
    print(f"  Date range: {args.start} to {args.end}")
    print(f"  Keyword: {args.keyword or '(none)'}")
    print(f"  Limit: {args.limit}")

    # Fetch raw CVEs (preserving full NVD nesting)
    raw_vulns = fetch_cves(args.start, args.end, args.keyword, args.limit)

    if not raw_vulns:
        print("No CVEs found matching criteria.")
        return

    # Minimal cleanup: strip nulls and empty collections
    print("Cleaning records (stripping nulls/empties)...")
    vulns = [strip_empty(vuln) for vuln in raw_vulns]

    # Sort by published date
    vulns.sort(key=lambda v: v.get("cve", {}).get("published", ""))

    # Build summary
    summary = summarize(vulns)
    print(f"  Severity distribution: {summary['severityDistribution']}")
    print(f"  Unique CWEs: {summary['uniqueCwes']}")
    print(f"  Unique vendors: {summary['uniqueVendors']}")

    # Build output — wrap in metadata envelope, pass through NVD structure
    keyword_slug = args.keyword.replace(" ", "_") if args.keyword else "all"
    period = f"{args.start}_to_{args.end}"

    output = {
        "source": "NIST National Vulnerability Database (NVD)",
        "license": "US Government Public Domain",
        "url": "https://nvd.nist.gov",
        "api": "NVD CVE API v2.0",
        "description": (
            f"{len(vulns)} CVE vulnerability records"
            f"{f' matching keyword \"{args.keyword}\"' if args.keyword else ''}, "
            f"published {args.start} to {args.end}. "
            f"Original NVD nesting preserved: metrics → cvssMetricV31 → cvssData, "
            f"configurations → nodes → cpeMatch, weaknesses → description."
        ),
        "query": {
            "keyword": args.keyword,
            "pubStartDate": args.start,
            "pubEndDate": args.end,
            "fetchedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        },
        "summary": summary,
        "vulnerabilities": vulns,
    }

    # Write output
    out_path = DATA_DIR / f"nvd_cves_{keyword_slug}_{period}.json"
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(output, fh, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"CVEs: {len(vulns)}")
    print(f"Top vendors: {list(summary['topVendors'].keys())[:5]}")


if __name__ == "__main__":
    main()
