# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0

require "ostruct"
require "pony"
require "sinatra"
require 'json'
require 'time'  # for ISO 8601 formatting

require "opentelemetry/sdk"
require "opentelemetry/exporter/otlp"
require "opentelemetry/instrumentation/sinatra"

set :port, ENV["EMAIL_PORT"]

OpenTelemetry::SDK.configure do |c|
  c.use "OpenTelemetry::Instrumentation::Sinatra"
end

# Log application startup in JSON format
startup_log = {
  time: Time.now.utc.iso8601(3),
  message: "Email service starting on port #{ENV["EMAIL_PORT"]}",
  level: "INFO"
}
puts startup_log.to_json

post "/send_order_confirmation" do
  data = JSON.parse(request.body.read, object_class: OpenStruct)

  # get the current auto-instrumented span
  current_span = OpenTelemetry::Trace.current_span
  current_span.add_attributes({
    "app.order.id" => data.order.order_id,
  })

  send_email(data)

end

error do
  error = env['sinatra.error']
  span = OpenTelemetry::Trace.current_span
  span.record_exception(error)
  
  # Log error in JSON format
  span_context = span.context
  error_log = {
    time: Time.now.utc.iso8601(3),
    message: "Error in email service: #{error.message}",
    trace_id: span_context.trace_id.unpack1('H*'),
    span_id: span_context.span_id.unpack1('H*'),
    level: "ERROR",
    error: error.message,
    backtrace: error.backtrace&.join("\n")
  }
  puts error_log.to_json
end

def send_email(data)
  # create and start a manual span
  tracer = OpenTelemetry.tracer_provider.tracer('email')
  tracer.in_span("send_email") do |span|
    # Log before sending email (in JSON format)
    span_context = span.context
    start_log = {
      time: Time.now.utc.iso8601(3),
      message: "Starting to send order confirmation email to: #{data.email}",
      trace_id: span_context.trace_id.unpack1('H*'),
      span_id: span_context.span_id.unpack1('H*'),
      level: "INFO"
    }
    puts start_log.to_json

    Pony.mail(
      to:       data.email,
      from:     "noreply@example.com",
      subject:  "Your confirmation email",
      body:     erb(:confirmation, locals: { order: data.order }),
      via:      :test
    )
    span.set_attribute("app.email.recipient", data.email)
    # Get current span context
    span_context = span.context

    # Prepare log data
    log_entry = {
      time: Time.now.utc.iso8601(3),  # ISO 8601 with milliseconds
      message: "Order confirmation email sent to: #{data.email}",
      trace_id: span_context.trace_id.unpack1('H*'),  # hex string
      span_id: span_context.span_id.unpack1('H*'),     # hex string
      level: "INFO"
    }

    # Print JSON log
    puts log_entry.to_json
  end
  # manually created spans need to be ended
  # in Ruby, the method `in_span` ends it automatically
  # check out the OpenTelemetry Ruby docs at: 
  # https://opentelemetry.io/docs/instrumentation/ruby/manual/#creating-new-spans 
end
