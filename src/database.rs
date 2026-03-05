use chrono::NaiveDateTime;
use tiberius::{Client, Config, AuthMethod, Row};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub struct PurchaseOrderLine {
    pub sku_number: String,
    pub description: Option<String>,
    pub warehouse_name: Option<String>,
    pub location_stock_type: String,
    pub customer: Option<String>,
    pub customer_sku: Option<String>,
    pub origin: Option<String>,
    pub date: NaiveDateTime,
    pub square_footage_per_crate: Option<f64>,
    pub number_of_crates: Option<f64>,
    pub pieces_per_crate: Option<String>,
    pub weight_per_crate_lbs: Option<String>,
}

pub async fn connect(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    database_name: &str,
) -> Result<Client<tokio_util::compat::Compat<TcpStream>>, tiberius::error::Error> {
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.authentication(AuthMethod::sql_server(user, password));
    config.database(database_name);
    config.trust_cert();

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true).ok();
    Client::connect(config, tcp.compat_write()).await
}

const PURCHASE_ORDER_QUERY: &str = "SELECT
    CAST(po.ItemCode AS NVARCHAR(MAX)) AS SKU_NUMBER,
    CAST(po.FreeTxt AS NVARCHAR(MAX)) AS DESCRIPTION,
    CAST(whs.WhsName AS NVARCHAR(MAX)) AS WAREHOUSE_NAME,
    CAST('SPECIAL ORDER' AS NVARCHAR(MAX)) AS LOCATION_STOCKTYPE,
    CAST(so.CardName AS NVARCHAR(MAX)) AS CUSTOMER,
    CASE
        WHEN po.U_TRC_LNum IS NOT NULL
        THEN CAST((SELECT TOP 1 Substitute FROM OSCN WHERE ItemCode = po.ItemCode AND CardCode = so.CardCode) AS NVARCHAR(MAX))
        ELSE NULL
    END AS CUSTOMER_SKU,
    CAST(po.CountryOrg AS NVARCHAR(MAX)) AS ORIGIN,
    poh.TaxDate AS DATE,
    CAST(itm.U_ses_sqfCrates AS FLOAT) AS SQUARE_FOOTAGE_PER_CRATE,
    CAST(po.U_ses_Crates AS FLOAT) AS NUMBER_OF_CRATES,
    CAST(itm.U_TRC_PkSz AS NVARCHAR(MAX)) AS PIECES_PER_CRATE,
    CAST(itm.U_weight_crate AS NVARCHAR(MAX)) AS WEIGHT_PER_CRATE_LBS
FROM
    POR1 AS po
    LEFT JOIN OWHS AS whs ON po.WhsCode = whs.WhsCode
    LEFT JOIN ORDR AS so ON po.U_TRC_LNum = so.NumAtCard
    LEFT JOIN OITM AS itm ON po.ItemCode = itm.ItemCode
    INNER JOIN OPOR AS poh ON po.DocEntry = poh.DocEntry
WHERE
    poh.DocNum = @P1";

pub async fn query_purchase_order_lines(
    client: &mut Client<tokio_util::compat::Compat<TcpStream>>,
    purchase_order_number: i32,
) -> Result<Vec<PurchaseOrderLine>, tiberius::error::Error> {
    let rows = client
        .query(PURCHASE_ORDER_QUERY, &[&purchase_order_number])
        .await?
        .into_first_result()
        .await?;

    let lines = rows.iter().map(parse_row).collect();
    Ok(lines)
}

fn parse_row(row: &Row) -> PurchaseOrderLine {
    PurchaseOrderLine {
        sku_number: row.get::<&str, _>(0).unwrap_or("").to_string(),
        description: row.get::<&str, _>(1).map(str::to_string),
        warehouse_name: row.get::<&str, _>(2).map(str::to_string),
        location_stock_type: row.get::<&str, _>(3).unwrap_or("").to_string(),
        customer: row.get::<&str, _>(4).map(str::to_string),
        customer_sku: row.get::<&str, _>(5).map(str::to_string),
        origin: row.get::<&str, _>(6).map(str::to_string),
        date: row.get::<NaiveDateTime, _>(7).unwrap_or_default(),
        square_footage_per_crate: row.get::<f64, _>(8),
        number_of_crates: row.get::<f64, _>(9),
        pieces_per_crate: row.get::<&str, _>(10).map(str::to_string),
        weight_per_crate_lbs: row.get::<&str, _>(11).map(str::to_string),
    }
}
