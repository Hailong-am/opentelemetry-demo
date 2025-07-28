// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use opentelemetry::global;
use opentelemetry_instrumentation_actix_web::ClientExt;
use std::{collections::HashMap, env, time::Instant};

use anyhow::{Context, Result};
use opentelemetry::{trace::get_active_span, KeyValue};
use tracing::{info, error, warn, instrument};

use super::shipping_types::Quote;

#[instrument(name = "shipping.create_quote_from_count", fields(item_count = count))]
pub async fn create_quote_from_count(count: u32) -> Result<Quote, tonic::Status> {
    info!(
        service = "shipping",
        operation = "create_quote_from_count",
        item_count = count,
        "Starting quote calculation for items"
    );

    let quote_value = match request_quote(count).await {
        Ok(value) => {
            info!(
                service = "shipping",
                operation = "create_quote_from_count",
                item_count = count,
                raw_quote_value = value,
                "Successfully received quote from external service"
            );
            value
        }
        Err(err) => {
            error!(
                service = "shipping",
                operation = "create_quote_from_count",
                item_count = count,
                error = %err,
                "Failed to get quote from external service"
            );
            return Err(tonic::Status::internal(format!(
                "Quote service unavailable: {}", err
            )));
        }
    };

    // Record metrics
    let meter = global::meter("otel_demo.shipping.quote");
    let counter = meter.u64_counter("app.shipping.items_count").build();
    counter.add(count as u64, &[]);

    let quote = get_active_span(|span| {
        let q = create_quote_from_float(quote_value);
        
        // Add span events and attributes
        span.add_event(
            "Quote Calculated".to_string(),
            vec![
                KeyValue::new("app.shipping.cost.total", format!("{}", q)),
                KeyValue::new("app.shipping.cost.dollars", q.dollars as i64),
                KeyValue::new("app.shipping.cost.cents", q.cents as i64),
                KeyValue::new("app.shipping.items.count", count as i64),
            ],
        );
        span.set_attribute(KeyValue::new("app.shipping.cost.total", format!("{}", q)));
        span.set_attribute(KeyValue::new("app.shipping.items.count", count as i64));
        
        info!(
            service = "shipping",
            operation = "create_quote_from_count",
            item_count = count,
            quote_dollars = q.dollars,
            quote_cents = q.cents,
            quote_total = %q,
            "Quote calculation completed successfully"
        );
        
        q
    });

    Ok(quote)
}

#[instrument(name = "shipping.request_quote", fields(item_count = count))]
async fn request_quote(count: u32) -> Result<f64, anyhow::Error> {
    let start_time = Instant::now();
    
    // Build quote service address
    let quote_service_addr = format!(
        "{}{}",
        env::var("QUOTE_ADDR")
            .unwrap_or_else(|_| "http://quote:8090".to_string()),
        "/getquote"
    );

    info!(
        service = "shipping",
        operation = "request_quote",
        item_count = count,
        quote_service_addr = quote_service_addr.as_str(),
        "Requesting quote from external service"
    );

    // Validate item count
    if count == 0 {
        warn!(
            service = "shipping",
            operation = "request_quote",
            item_count = count,
            "Requesting quote for zero items"
        );
    }

    let client = awc::Client::new();
    let mut request_body = HashMap::new();
    request_body.insert("numberOfItems", count);

    // Make HTTP request
    let mut response = client
        .post(&quote_service_addr)
        .trace_request()
        .send_json(&request_body)
        .await
        .map_err(|err| {
            error!(
                service = "shipping",
                operation = "request_quote",
                item_count = count,
                quote_service_addr = quote_service_addr.as_str(),
                error = %err,
                duration_ms = start_time.elapsed().as_millis(),
                "Failed to send request to quote service"
            );
            anyhow::anyhow!("HTTP request failed: {}", err)
        })?;

    // Check response status
    let status = response.status();
    if !status.is_success() {
        error!(
            service = "shipping",
            operation = "request_quote",
            item_count = count,
            quote_service_addr = quote_service_addr.as_str(),
            status_code = status.as_u16(),
            duration_ms = start_time.elapsed().as_millis(),
            "Quote service returned error status"
        );
        return Err(anyhow::anyhow!("Quote service returned status: {}", status));
    }

    // Read response body
    let bytes = response
        .body()
        .await
        .map_err(|err| {
            error!(
                service = "shipping",
                operation = "request_quote",
                item_count = count,
                error = %err,
                duration_ms = start_time.elapsed().as_millis(),
                "Failed to read response body from quote service"
            );
            anyhow::anyhow!("Failed to read response body: {}", err)
        })?;

    // Parse response as UTF-8
    let response_text = std::str::from_utf8(&bytes)
        .context("Quote service response is not valid UTF-8")?
        .trim();

    // Parse quote value
    let quote_value = response_text
        .parse::<f64>()
        .map_err(|err| {
            error!(
                service = "shipping",
                operation = "request_quote",
                item_count = count,
                response_text = response_text,
                error = %err,
                duration_ms = start_time.elapsed().as_millis(),
                "Failed to parse quote value as number"
            );
            anyhow::anyhow!("Invalid quote format '{}': {}", response_text, err)
        })?;

    // Validate quote value
    if quote_value < 0.0 {
        warn!(
            service = "shipping",
            operation = "request_quote",
            item_count = count,
            quote_value = quote_value,
            duration_ms = start_time.elapsed().as_millis(),
            "Received negative quote value"
        );
    }

    info!(
        service = "shipping",
        operation = "request_quote",
        item_count = count,
        quote_value = quote_value,
        duration_ms = start_time.elapsed().as_millis(),
        "Successfully received quote from external service"
    );

    Ok(quote_value)
}

#[instrument(name = "shipping.create_quote_from_float", fields(raw_value = value))]
pub fn create_quote_from_float(value: f64) -> Quote {
    let quote = Quote {
        dollars: value.floor() as u64,
        cents: ((value * 100_f64) as u32) % 100,
    };

    info!(
        service = "shipping",
        operation = "create_quote_from_float",
        raw_value = value,
        quote_dollars = quote.dollars,
        quote_cents = quote.cents,
        quote_total = %quote,
        "Converted float value to structured quote"
    );

    quote
}

impl fmt::Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.dollars, self.cents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_quote_from_float() {
        let quote = create_quote_from_float(10.99);
        assert_eq!(quote.dollars, 10);
        assert_eq!(quote.cents, 99);

        let quote = create_quote_from_float(0.01);
        assert_eq!(quote.dollars, 0);
        assert_eq!(quote.cents, 1);

        let quote = create_quote_from_float(100.00);
        assert_eq!(quote.dollars, 100);
        assert_eq!(quote.cents, 0);
    }

    #[test]
    fn test_quote_display() {
        let quote = Quote {
            dollars: 10,
            cents: 99,
        };
        assert_eq!(format!("{}", quote), "10.99");

        let quote = Quote {
            dollars: 0,
            cents: 1,
        };
        assert_eq!(format!("{}", quote), "0.1");
    }
}
