# USDA FoodData Central — Branded Food Products

## Source

**USDA FoodData Central**
https://fdc.nal.usda.gov

## License

US Government Public Domain

## How This Data Was Produced

Branded food product records are fetched from the USDA FoodData Central API v1
with **minimal processing** — the native USDA response structure is preserved.
Each food contains a `foodNutrients` array with 14 fields per nutrient entry
(nutrientId, nutrientName, unitName, value, derivation codes,
percentDailyValue, etc.), repeated 9–86 times per food product.

### Steps

1. Query the FoodData Central API at `https://api.nal.usda.gov/fdc/v1/foods/search`
   with search term and `dataType=Branded`
2. Paginate through results (DEMO_KEY rate limit: 30 requests/hour)
3. Strip the API pagination envelope (`totalHits`, `currentPage`, `totalPages`,
   `pageList`, `foodSearchCriteria`, `aggregations`)
4. Recursively remove null values and empty collections
5. Sort alphabetically by product description
6. Wrap in metadata envelope with summary statistics

### Reproducing

```bash
# Install dependencies
pip install requests

# Fetch default dataset (breakfast cereal, 25 items)
python process_usda.py

# Custom query and limit
python process_usda.py --query "yogurt" --limit 30

# Larger dataset
python process_usda.py --query "snack bars" --limit 50
```

### Output Schema

The native USDA nesting is preserved — each food retains the original
`foods[] → foodNutrients[]` structure with 14 fields per nutrient entry:

```json
{
  "source": "USDA FoodData Central",
  "license": "US Government Public Domain",
  "url": "https://fdc.nal.usda.gov",
  "api": "FoodData Central API v1",
  "description": "25 branded food products matching ...",
  "query": { "searchTerm", "dataType", "fetchedAt" },
  "summary": {
    "totalFoods": 25,
    "totalNutrientEntries": 697,
    "uniqueNutrients": 34,
    "avgNutrientsPerFood": 27.9,
    "categoryDistribution": { "Processed Cereal Products": 21, ... },
    "topBrandOwners": { "GENERAL MILLS SALES INC.": 19, ... }
  },
  "foods": [
    {
      "fdcId": 2746583,
      "description": "Bluey Breakfast Cereal",
      "dataType": "Branded",
      "gtinUpc": "00016000227835",
      "publishedDate": "2025-11-20",
      "brandOwner": "GENERAL MILLS SALES INC.",
      "brandName": "Kix",
      "ingredients": "Corn Meal, Sugar, ...",
      "marketCountry": "US",
      "foodCategory": "Processed Cereal Products",
      "modifiedDate": "2025-10-21",
      "dataSource": "GDSN",
      "packageWeight": "12 ONZ",
      "servingSizeUnit": "GRM",
      "servingSize": 40.0,
      "householdServingFullText": "1 1/2 cup",
      "tradeChannels": ["NO_TRADE_CHANNEL"],
      "score": 208.49,
      "foodNutrients": [
        {
          "nutrientId": 1003,
          "nutrientName": "Protein",
          "nutrientNumber": "203",
          "unitName": "G",
          "derivationCode": "LCSA",
          "derivationDescription": "Calculated from ...",
          "derivationId": 71,
          "value": 7.5,
          "foodNutrientSourceId": 9,
          "foodNutrientSourceCode": "12",
          "foodNutrientSourceDescription": "Manufacturer's analytical; ...",
          "rank": 600,
          "indentLevel": 1,
          "foodNutrientId": 34935766,
          "percentDailyValue": 2
        }
      ]
    }
  ]
}
```

The high structural repetition in `foodNutrients` (same 14-field schema repeated
697 times across 25 foods) makes this dataset ideal for `@struct`/`@table`
compression. Key name repetition alone accounts for ~14,000 redundant key
occurrences, and constant-value fields like `foodNutrientSourceDescription`
repeat the same 53-character string hundreds of times.
