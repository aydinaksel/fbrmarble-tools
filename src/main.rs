#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{routing::get, Router};
    use fbr_tools::app::{shell, App};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    dotenvy::dotenv().ok();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/crate-labels/{purchase_order_number}", get(labels_handler))
        .leptos_routes(&leptos_options, routes, {
            let options = leptos_options.clone();
            move || shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    eprintln!("Listening on http://{}", addr);
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(feature = "ssr")]
async fn labels_handler(
    axum::extract::Path(purchase_order_number_string): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::{
        http::{header, StatusCode},
        response::IntoResponse,
    };
    use fbr_tools::{database, pdf};

    let purchase_order_number: i32 = match purchase_order_number_string.parse() {
        Ok(number) => number,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid purchase order number").into_response();
        }
    };

    let host = std::env::var("SAP_DB_HOST").unwrap_or_default();
    let port: u16 = std::env::var("SAP_DB_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1433);
    let user = std::env::var("SAP_DB_USER").unwrap_or_default();
    let password = std::env::var("SAP_DB_PASSWORD").unwrap_or_default();
    let database_name = std::env::var("SAP_DB_NAME").unwrap_or_default();

    let mut client =
        match database::connect(&host, port, &user, &password, &database_name).await {
            Ok(client) => client,
            Err(error) => {
                eprintln!("Database connection failed: {}", error);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to connect to database",
                )
                    .into_response();
            }
        };

    let lines =
        match database::query_purchase_order_lines(&mut client, purchase_order_number).await {
            Ok(lines) => lines,
            Err(error) => {
                eprintln!("Query failed for PO {}: {}", purchase_order_number, error);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to query purchase order",
                )
                    .into_response();
            }
        };

    let labels = pdf::expand_to_crate_labels(lines);
    if labels.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            "No crate labels found for this purchase order",
        )
            .into_response();
    }

    let pdf_bytes = pdf::generate_pdf(&labels);
    let filename = format!("PO-{}-crate-labels.pdf", purchase_order_number_string);

    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/pdf".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", filename),
            ),
        ],
        pdf_bytes,
    )
        .into_response()
}

#[cfg(not(feature = "ssr"))]
fn main() {}
