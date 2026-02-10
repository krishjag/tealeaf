use tealeaf::convert::NotU8;
use tealeaf_derive::{FromTeaLeaf, ToTeaLeaf};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "stock")]
pub struct StockInfo {
    pub warehouse: String,
    pub qty_available: i32,
    pub reorder_level: i32,
    pub backordered: bool,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "price")]
pub struct ProductPrice {
    pub base_price: f64,
    #[tealeaf(optional)]
    pub discount_pct: Option<f64>,
    pub currency: String,
    pub tax_rate: f64,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "product")]
pub struct Product {
    pub id: String,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub brand: String,
    pub price: ProductPrice,
    pub stock: Vec<StockInfo>,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "item")]
pub struct OrderItem {
    pub line_number: i32,
    pub product: Product,
    pub quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
    #[tealeaf(optional)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "shipping")]
pub struct ShippingAddress {
    pub name: String,
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

#[derive(Debug, Clone, ToTeaLeaf, FromTeaLeaf)]
#[tealeaf(rename = "order")]
pub struct Order {
    pub order_id: String,
    pub status: String,
    pub placed_at: String,
    #[tealeaf(optional)]
    pub shipped_at: Option<String>,
    pub customer_id: String,
    pub customer_email: String,
    pub shipping: ShippingAddress,
    pub items: Vec<OrderItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub shipping_cost: f64,
    pub total: f64,
    pub payment_method: String,
}

// Vec<T> requires T: NotU8 to distinguish from Vec<u8> (bytes)
impl NotU8 for StockInfo {}
impl NotU8 for OrderItem {}
impl NotU8 for Order {}

// ── Sample data ──────────────────────────────────────────────────────────────

pub fn sample_products() -> Vec<Product> {
    vec![
        Product {
            id: "PROD-7841".into(),
            name: "Sony WH-1000XM5 Wireless Headphones".into(),
            sku: "SNY-WH1000XM5-BLK".into(),
            category: "Electronics/Audio".into(),
            brand: "Sony".into(),
            price: ProductPrice {
                base_price: 399.99,
                discount_pct: Some(15.0),
                currency: "USD".into(),
                tax_rate: 8.25,
            },
            stock: vec![
                StockInfo {
                    warehouse: "US-WEST-01".into(),
                    qty_available: 342,
                    reorder_level: 50,
                    backordered: false,
                },
                StockInfo {
                    warehouse: "US-EAST-02".into(),
                    qty_available: 189,
                    reorder_level: 50,
                    backordered: false,
                },
            ],
        },
        Product {
            id: "PROD-2156".into(),
            name: "Anker USB-C to Lightning Cable 6ft".into(),
            sku: "ANK-USBC-LTN-6FT".into(),
            category: "Electronics/Accessories".into(),
            brand: "Anker".into(),
            price: ProductPrice {
                base_price: 15.99,
                discount_pct: None,
                currency: "USD".into(),
                tax_rate: 8.25,
            },
            stock: vec![StockInfo {
                warehouse: "US-WEST-01".into(),
                qty_available: 2841,
                reorder_level: 500,
                backordered: false,
            }],
        },
        Product {
            id: "PROD-9034".into(),
            name: "Patagonia Better Sweater Jacket - Navy L".into(),
            sku: "PAT-BSWEATER-NVY-L".into(),
            category: "Apparel/Outerwear".into(),
            brand: "Patagonia".into(),
            price: ProductPrice {
                base_price: 149.00,
                discount_pct: Some(20.0),
                currency: "USD".into(),
                tax_rate: 9.50,
            },
            stock: vec![
                StockInfo {
                    warehouse: "US-WEST-01".into(),
                    qty_available: 23,
                    reorder_level: 30,
                    backordered: false,
                },
                StockInfo {
                    warehouse: "US-EAST-02".into(),
                    qty_available: 0,
                    reorder_level: 30,
                    backordered: true,
                },
            ],
        },
        Product {
            id: "PROD-5512".into(),
            name: "Chemex 8-Cup Pour-Over Coffee Maker".into(),
            sku: "CHX-POUROVER-8CUP".into(),
            category: "Home/Kitchen".into(),
            brand: "Chemex".into(),
            price: ProductPrice {
                base_price: 44.95,
                discount_pct: None,
                currency: "USD".into(),
                tax_rate: 8.25,
            },
            stock: vec![StockInfo {
                warehouse: "US-EAST-02".into(),
                qty_available: 567,
                reorder_level: 100,
                backordered: false,
            }],
        },
    ]
}

pub fn sample_orders() -> Vec<Order> {
    let products = sample_products();

    vec![
        Order {
            order_id: "ORD-2025-00847".into(),
            status: "shipped".into(),
            placed_at: "2025-06-14T09:23:17Z".into(),
            shipped_at: Some("2025-06-15T14:02:00Z".into()),
            customer_id: "CUST-30291".into(),
            customer_email: "marcus.chen@gmail.com".into(),
            shipping: ShippingAddress {
                name: "Marcus Chen".into(),
                street: "4521 Olive Branch Ln".into(),
                city: "Portland".into(),
                state: "OR".into(),
                zip: "97201".into(),
                country: "US".into(),
            },
            items: vec![
                OrderItem {
                    line_number: 1,
                    product: products[0].clone(),
                    quantity: 1,
                    unit_price: 339.99, // after 15% discount
                    line_total: 339.99,
                    notes: Some("Gift wrap requested".into()),
                },
                OrderItem {
                    line_number: 2,
                    product: products[1].clone(),
                    quantity: 2,
                    unit_price: 15.99,
                    line_total: 31.98,
                    notes: None,
                },
            ],
            subtotal: 371.97,
            tax: 30.69,
            shipping_cost: 0.00,
            total: 402.66,
            payment_method: "visa_ending_4242".into(),
        },
        Order {
            order_id: "ORD-2025-00848".into(),
            status: "processing".into(),
            placed_at: "2025-06-14T11:47:53Z".into(),
            shipped_at: None,
            customer_id: "CUST-18774".into(),
            customer_email: "sofia.r@outlook.com".into(),
            shipping: ShippingAddress {
                name: "Sofia Ramirez".into(),
                street: "88 Beacon St Apt 3F".into(),
                city: "Boston".into(),
                state: "MA".into(),
                zip: "02108".into(),
                country: "US".into(),
            },
            items: vec![
                OrderItem {
                    line_number: 1,
                    product: products[2].clone(),
                    quantity: 1,
                    unit_price: 119.20, // after 20% discount
                    line_total: 119.20,
                    notes: None,
                },
                OrderItem {
                    line_number: 2,
                    product: products[3].clone(),
                    quantity: 1,
                    unit_price: 44.95,
                    line_total: 44.95,
                    notes: Some("Include Chemex filters sample pack".into()),
                },
                OrderItem {
                    line_number: 3,
                    product: products[1].clone(),
                    quantity: 3,
                    unit_price: 15.99,
                    line_total: 47.97,
                    notes: None,
                },
            ],
            subtotal: 212.12,
            tax: 18.90,
            shipping_cost: 7.99,
            total: 239.01,
            payment_method: "paypal_sofia.r@outlook.com".into(),
        },
    ]
}