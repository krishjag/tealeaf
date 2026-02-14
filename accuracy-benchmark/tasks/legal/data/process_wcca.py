#!/usr/bin/env python3
"""Process Wisconsin Circuit Court Access (WCCA) case data into a benchmark JSON file.

Unlike other benchmark data scripts that download from public APIs, WCCA requires
solving an hCaptcha for each case detail request. The raw JSON must be manually
captured from the browser and placed in this directory.

To capture a case:
  1. Open https://wcca.wicourts.gov/caseDetail.html?caseNo=<CASE>&countyNo=<COUNTY>&mode=details
  2. Open browser DevTools (F12) → Network tab
  3. Solve the CAPTCHA; the browser will POST to caseDetail/<countyNo>/<caseNo>
  4. Copy the JSON response and save as wisconsin_circuit_court_<DA_CASE_NO>.json

This script reads manually captured case files, strips internal UI fields,
and produces a clean hierarchical JSON suitable for benchmarking.

Usage:
    python process_wcca.py                          # process all case files
    python process_wcca.py --cases 2015CO001313     # specific case(s)

Requirements:
    pip install polars  (only for consistency with other scripts; uses stdlib mostly)
"""

import argparse
import json
import re
from pathlib import Path

DATA_DIR = Path(__file__).parent

# Fields to strip from the output (UI-only, not analytically useful)
STRIP_TOP_LEVEL = {
    "available", "showRssButton", "tac", "allowPurchase", "lienOnlyCaseType",
    "payplanLink", "documents", "maintenance", "isReopenedRemandedFromAppeal",
}

STRIP_RECORD_FIELDS = {"key"}
STRIP_CHARGE_FIELDS = {"key", "id", "isSummaryCandidate"}
STRIP_JUDGMENT_FIELDS = {"key", "isConverted", "reOpenedEvent", "probByCase", "superAgency"}
STRIP_SUPERVISION_FIELDS = {"key", "isShowConds", "superTimeConds", "beginDate"}
STRIP_COND_FIELDS = {"key"}
STRIP_MODIFIER_FIELDS = set()
STRIP_ATTY_FIELDS = {"key"}
STRIP_ALIAS_FIELDS = {"key"}
STRIP_PARTY_FIELDS = {"key"}
STRIP_CHARGE_HIST_FIELDS = {"key", "id", "isConverted"}
STRIP_RECEIVABLE_FIELDS = {"key"}


def clean_record(rec: dict) -> dict:
    """Clean a docket record entry."""
    out = {k: v for k, v in rec.items() if k not in STRIP_RECORD_FIELDS}
    # Clean parties within records
    if "parties" in out and not out["parties"]:
        del out["parties"]
    elif "parties" in out:
        out["parties"] = [
            {k: v for k, v in p.items() if k not in STRIP_PARTY_FIELDS}
            for p in out["parties"]
        ]
    return out


def clean_supervision(sup: dict) -> dict:
    """Clean a supervision entry."""
    out = {k: v for k, v in sup.items() if k not in STRIP_SUPERVISION_FIELDS}
    if "superMiscConds" in out:
        out["conditions"] = [
            {k: v for k, v in c.items() if k not in STRIP_COND_FIELDS}
            for c in out["superMiscConds"]
        ]
        del out["superMiscConds"]
    return out


def clean_judgment(jdg: dict) -> dict:
    """Clean a judgment entry."""
    out = {k: v for k, v in jdg.items() if k not in STRIP_JUDGMENT_FIELDS}
    if "supervisions" in out:
        out["supervisions"] = [clean_supervision(s) for s in out["supervisions"]]
    if "miscConds" in out and not out["miscConds"]:
        del out["miscConds"]
    if "timeConds" in out and not out["timeConds"]:
        del out["timeConds"]
    return out


def clean_charge(ch: dict) -> dict:
    """Clean a charge entry."""
    out = {k: v for k, v in ch.items() if k not in STRIP_CHARGE_FIELDS}
    if "judgments" in out:
        out["judgments"] = [clean_judgment(j) for j in out["judgments"]]
    if "chargeModifiers" in out:
        out["modifiers"] = [
            {k: v for k, v in m.items() if k not in STRIP_MODIFIER_FIELDS}
            for m in out["chargeModifiers"]
        ]
        del out["chargeModifiers"]
    return out


def clean_defendant(deft: dict) -> dict:
    """Clean defendant info."""
    if not deft:
        return deft
    out = dict(deft)
    if "attys" in out:
        out["attorneys"] = [
            {k: v for k, v in a.items() if k not in STRIP_ATTY_FIELDS}
            for a in out["attys"]
        ]
        del out["attys"]
    if "alias" in out:
        out["aliases"] = [
            {k: v for k, v in a.items() if k not in STRIP_ALIAS_FIELDS}
            for a in out["alias"]
        ]
        del out["alias"]
    # Remove internal fields
    for f in ("sealed", "inclGal", "isDobSealed", "justisNo", "fingerprintId", "partyNo"):
        out.pop(f, None)
    return out


def clean_charge_hist(ch: dict) -> dict:
    """Clean a charge history entry."""
    return {k: v for k, v in ch.items() if k not in STRIP_CHARGE_HIST_FIELDS}


def clean_receivable(rcv: dict) -> dict:
    """Clean a receivable entry."""
    return {k: v for k, v in rcv.items() if k not in STRIP_RECEIVABLE_FIELDS}


def strip_html(text: str) -> str:
    """Remove HTML tags from exec summary."""
    if not text:
        return text
    text = re.sub(r'<[^>]+>', ' ', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text


def process_case(raw: dict) -> dict:
    """Process a single raw WCCA case JSON into clean benchmark format."""
    # The raw JSON may have a 'result' wrapper
    r = raw.get("result", raw)

    case = {
        "county": r["countyName"],
        "countyNo": r["countyNo"],
        "caseNo": r["caseNo"],
        "daCaseNo": r.get("daCaseNo"),
        "caseType": r["caseType"],
        "classType": r["classType"],
        "caption": r["caption"],
        "status": r["status"],
        "filingDate": r["filingDate"],
        "isCriminal": r["isCriminal"],
        "wcisClsCode": r.get("wcisClsCode"),
        "branchId": r.get("branchId"),
        "respCtofc": r.get("respCtofc"),
        "prosAtty": r.get("prosAtty"),
        "prosAgency": r.get("prosAgency"),
        "balanceDue": r.get("balanceDue"),
    }

    # Executive summary (strip HTML)
    if r.get("execSummary"):
        case["execSummary"] = strip_html(r["execSummary"])

    # Defendant
    if r.get("defendant"):
        case["defendant"] = clean_defendant(r["defendant"])

    # Defense attorneys (top-level convenience list)
    if r.get("defAttys"):
        case["defenseAttorneys"] = r["defAttys"]

    # Charges (current)
    case["charges"] = [clean_charge(ch) for ch in r.get("charges", [])]

    # Charge history (shows original/amended charges)
    if r.get("chargeHist"):
        case["chargeHistory"] = [clean_charge_hist(ch) for ch in r["chargeHist"]]

    # Docket records (sorted chronologically, oldest first)
    records = [clean_record(rec) for rec in r.get("records", [])]
    records.sort(key=lambda x: x.get("date") or "")
    case["docketEntries"] = records

    # Receivables
    if r.get("receivables"):
        case["receivables"] = [clean_receivable(rcv) for rcv in r["receivables"]]

    # Warrants
    if r.get("warrants"):
        case["warrants"] = r["warrants"]

    # Civil judgments
    if r.get("civilJdgmts"):
        case["civilJudgments"] = r["civilJdgmts"]

    # Cross-referenced cases
    if r.get("crossReferenced"):
        case["crossReferenced"] = r["crossReferenced"]

    # Citations
    if r.get("citations"):
        case["citations"] = r["citations"]

    return case


def main():
    parser = argparse.ArgumentParser(
        description="Process WCCA case JSON files into benchmark format"
    )
    parser.add_argument(
        "--cases", nargs="*",
        help="DA case numbers to process (default: all wisconsin_circuit_court_*.json files)"
    )
    args = parser.parse_args()

    # Find input files
    if args.cases:
        files = []
        for case_no in args.cases:
            pattern = f"wisconsin_circuit_court_{case_no}.json"
            matches = list(DATA_DIR.glob(pattern))
            if not matches:
                print(f"Warning: No file found for case {case_no}")
            files.extend(matches)
    else:
        files = sorted(DATA_DIR.glob("wisconsin_circuit_court_*.json"))

    if not files:
        print("No case files found.")
        return

    print(f"Processing {len(files)} case file(s)...")

    cases = []
    for f in files:
        print(f"  Reading {f.name}...")
        with open(f, encoding="utf-8") as fh:
            raw = json.load(fh)
        case = process_case(raw)
        cases.append(case)
        n_charges = len(case["charges"])
        n_records = len(case["docketEntries"])
        print(f"    {case['caption']}: {n_charges} charges, {n_records} docket entries")

    # Build output
    output = {
        "source": "Wisconsin Circuit Court Access (WCCA)",
        "url": "https://wcca.wicourts.gov",
        "license": "Public Record — Wisconsin Supreme Court Rule 72",
        "description": (
            f"Circuit court case data from {len(cases)} Wisconsin criminal case(s). "
            f"Includes charges with nested sentencing structures, docket entries, "
            f"defendant information, and financial receivables."
        ),
        "note": (
            "Data manually captured from WCCA (requires hCaptcha). "
            "See README.md for reproduction steps."
        ),
        "case_count": len(cases),
        "cases": cases,
    }

    # Write output
    out_path = DATA_DIR / "wcca_cases.json"
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(output, fh, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"Cases: {len(cases)}")
    total_charges = sum(len(c["charges"]) for c in cases)
    total_records = sum(len(c["docketEntries"]) for c in cases)
    print(f"Total charges: {total_charges}")
    print(f"Total docket entries: {total_records}")


if __name__ == "__main__":
    main()
