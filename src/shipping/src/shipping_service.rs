// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use actix_web::{post, web, HttpResponse, Responder};
use tracing::{info, error, instrument};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::Context;
use serde_json::json;

mod quote;
use quote::create_quote_from_count;

mod tracking;
use tracking::create_tracking_id;

mod shipping_types;
pub use shipping_types::*;

const NANOS_MULTIPLE: u32 = 10000000u32;

// Helper function to extract trace context for consistent logging
fn get_trace_context() -> (String, String) {
    let current_context = Context::current();
    let current_span = current_context.span();
    let span_context = current_span.span_context();
    let trace_id = span_context.trace_id().to_string();
    let span_id = span_context.span_id().to_string();
    (trace_id, span_id)
}

#[post("/get-quote")]
#[instrument(name = "shipping.get_quote", skip(req))]
pub async fn get_quote(req: web::Json<GetQuoteRequest>) -> impl Responder {
    let item_count: u32 = req.items.iter().map(|item| item.quantity as u32).sum();
    let (trace_id, span_id) = get_trace_context();

    // Log incoming request with business context
    info!(
        service = "shipping",
        operation = "get_quote",
        item_count = item_count,
        has_address = req.address.is_some(),
        zip_code = req.address.as_ref().map(|a| a.zip_code.as_str()).unwrap_or("none"),
        trace_id = trace_id.as_str(),
        span_id = span_id.as_str(),
        "Processing shipping quote request"
    );

    let quote = match create_quote_from_count(item_count).await {
        Ok(q) => {
            info!(
                service = "shipping",
                operation = "get_quote",
                quote_dollars = q.dollars,
                quote_cents = q.cents,
                item_count = item_count,
                trace_id = trace_id.as_str(),
                span_id = span_id.as_str(),
                "Successfully calculated shipping quote"
            );
            q
        }
        Err(e) => {
            error!(
                service = "shipping",
                operation = "get_quote",
                error = %e,
                item_count = item_count,
                trace_id = trace_id.as_str(),
                span_id = span_id.as_str(),
                "Failed to calculate shipping quote"
            );
            return HttpResponse::InternalServerError()
                .json(json!({
                    "error": "Failed to calculate shipping quote",
                    "trace_id": trace_id
                }));
        }
    };

    let reply = GetQuoteResponse {
        cost_usd: Some(Money {
            currency_code: "USD".into(),
            units: quote.dollars,
            nanos: quote.cents * NANOS_MULTIPLE,
        }),
    };

    info!(
        service = "shipping",
        operation = "get_quote",
        response_dollars = quote.dollars,
        response_cents = quote.cents,
        currency = "USD",
        trace_id = trace_id.as_str(),
        span_id = span_id.as_str(),
        "Shipping quote response sent successfully"
    );

    HttpResponse::Ok().json(reply)
}

#[post("/ship-order")]
#[instrument(name = "shipping.ship_order", skip(req))]
pub async fn ship_order(req: web::Json<ShipOrderRequest>) -> impl Responder {
    let (trace_id, span_id) = get_trace_context();

    info!(
        service = "shipping",
        operation = "ship_order",
        trace_id = trace_id.as_str(),
        span_id = span_id.as_str(),
        "Processing ship order request"
    );

    let tracking_id = create_tracking_id();

    info!(
        service = "shipping",
        operation = "ship_order",
        tracking_id = tracking_id.as_str(),
        trace_id = trace_id.as_str(),
        span_id = span_id.as_str(),
        "Order shipped successfully with tracking ID"
    );

    HttpResponse::Ok().json(ShipOrderResponse { tracking_id })
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::ContentType, test, App};

    use super::*;

    #[actix_web::test]
    async fn test_ship_order() {
        let app = test::init_service(App::new().service(ship_order)).await;
        let req = test::TestRequest::post()
            .uri("/ship-order")
            .insert_header(ContentType::json())
            .set_json(&ShipOrderRequest {})
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let order: ShipOrderResponse = test::read_body_json(resp).await;
        assert!(!order.tracking_id.is_empty());
    }
}
