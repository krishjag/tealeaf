# Retail Order Analysis with Claude API
# Sends order data in PAX format for business intelligence analysis

$apiKey = [Environment]::GetEnvironmentVariable('ANTHROPIC_API_KEY', 'User')

# Read the PAX file content
$paxContent = Get-Content -Path "examples/retail_orders.pax" -Raw

# Use actual file content (or truncated sample for large files)
$paxSample = $paxContent

$body = @{
    model = "claude-sonnet-4-20250514"
    max_tokens = 2048
    messages = @(
        @{
            role = "user"
            content = @"
You are a retail business analyst. I'm providing you order data from our e-commerce system in PAX format (a schema-aware data format).

Analyze this data and provide:
1. **Executive Summary**: Key metrics and business health (2-3 sentences)
2. **Revenue Analysis**: Breakdown by product category and customer segment
3. **Order Patterns**: Any trends in order status, payment methods, or shipping
4. **Customer Insights**: Analysis of the 3 customers (B2C vs B2B behavior)
5. **Recommendations**: 3 actionable business recommendations based on the data

Here's the order data:

$paxSample
"@
        }
    )
} | ConvertTo-Json -Depth 10

Write-Host "=== Sending Retail Order Data to Claude for Analysis ===" -ForegroundColor Cyan
Write-Host "Data: 10 orders, 4 products, 3 customers, 11 schemas"
Write-Host "Binary size: 6.9 KB (35% of original 19.6 KB text)"
Write-Host ""

$response = Invoke-RestMethod -Uri "https://api.anthropic.com/v1/messages" `
    -Method Post `
    -Headers @{
        "x-api-key" = $apiKey
        "anthropic-version" = "2023-06-01"
        "content-type" = "application/json"
    } `
    -Body $body

Write-Host "=== Claude's Retail Analysis ===" -ForegroundColor Green
Write-Host "Usage: input=$($response.usage.input_tokens), output=$($response.usage.output_tokens) tokens"
Write-Host ""
Write-Host $response.content[0].text

# Save responses
$response | ConvertTo-Json -Depth 10 | Out-File -FilePath "examples/responses/retail_analysis.json" -Encoding UTF8

$paxOutput = @"
# Claude's Retail Order Analysis
# Generated: $(Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
# Source: examples/retail_orders.pax

analysis: {
  model: $($response.model),
  input_tokens: $($response.usage.input_tokens),
  output_tokens: $($response.usage.output_tokens),
  content: """
$($response.content[0].text)
""",
}
"@
$paxOutput | Out-File -FilePath "examples/responses/retail_analysis.pax" -Encoding UTF8

Write-Host ""
Write-Host "Saved to examples/responses/retail_analysis.json and .pax" -ForegroundColor Cyan
