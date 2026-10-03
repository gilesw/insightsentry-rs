use insightsentry::{
    Client, Error, GetV3DocumentsIdResponse, GetV3SymbolsSymbolHistoryBarType,
    GetV3SymbolsSymbolSeriesBarType, GetV3SymbolsSymbolSeriesCurrency, HistoryQuery, QuoteQuery,
    QuoteResult, SearchQuery, SeriesQuery,
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn client(server: &MockServer) -> Client {
    Client::new("direct-secret").with_base_url(format!("{}/", server.uri()))
}
fn quote() -> Value {
    json!({"code":"NASDAQ:AAPL","last_price":123.456,"lp_time":1738022400,"status":"CLOSED"})
}
fn quotes() -> Value {
    json!({"data":[quote()],"total_items":1})
}
fn series() -> Value {
    json!({"code":"NASDAQ:AAPL","last_update":1738022400000i64,"bar_type":"day","series":[{"time":1738022400,"close":123.456}]})
}
fn quote_error() -> Value {
    json!({"code":"NASDAQ:BAD","message":"Unknown symbol","error":"invalid_symbol"})
}
fn no_rapid_auth(request: &Request) -> bool {
    !request.headers.contains_key("x-rapidapi-key")
        && !request.headers.contains_key("x-rapidapi-host")
}
async fn respond(server: &MockServer, route: &str, body: Value) {
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn bearer_auth_and_explicit_default_settlement() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/quotes"))
        .and(header("authorization", "Bearer direct-secret"))
        .and(no_rapid_auth)
        .and(query_param("codes", "NASDAQ:AAPL"))
        .and(query_param("settlement", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(quotes()))
        .expect(1)
        .mount(&server)
        .await;
    let q = client(&server).quote("NASDAQ:AAPL").await.unwrap().unwrap();
    assert_eq!(q.last_price, Some(123.456));
    assert!(
        !server.received_requests().await.unwrap()[0]
            .url
            .as_str()
            .contains("direct-secret")
    );
}

#[tokio::test]
async fn batch_retains_successes_and_per_symbol_errors() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/quotes"))
        .and(query_param("codes", "NASDAQ:AAPL,NASDAQ:BAD"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[quote(),quote_error()],"total_items":2})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let batch = client(&server)
        .quotes(&["NASDAQ:AAPL", "NASDAQ:BAD"])
        .await
        .unwrap();
    assert_eq!(batch.total_items, 2);
    assert!(matches!(&batch.data[0],QuoteResult::QuoteData(q) if q.last_price==Some(123.456)));
    assert!(matches!(&batch.data[1],QuoteResult::QuoteError(e) if e.message=="Unknown symbol"));
}

#[tokio::test]
async fn single_quote_provider_error_is_not_missing_data() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v3/symbols/quotes",
        json!({"data":[quote_error()],"total_items":1}),
    )
    .await;
    assert!(matches!(client(&server).quote("NASDAQ:BAD").await,
        Err(Error::Quote {code,message,reason}) if code=="NASDAQ:BAD" && message=="Unknown symbol" && reason.as_deref()==Some("invalid_symbol")));
}

#[tokio::test]
async fn unknown_quote_error_codes_are_preserved() {
    let server = MockServer::start().await;
    let mut error = quote_error();
    error["error"] = json!("new_provider_error");
    respond(
        &server,
        "/v3/symbols/quotes",
        json!({"data":[error],"total_items":1}),
    )
    .await;
    assert!(
        matches!(client(&server).quote("NASDAQ:BAD").await,Err(Error::Quote {reason,..}) if reason.as_deref()==Some("new_provider_error"))
    );
}

#[tokio::test]
async fn missing_symbol_missing_price_and_invalid_envelope_are_distinct() {
    for (body, case) in [
        (json!({"data":[],"total_items":0}), 0),
        (json!({"data":[{"code":"NASDAQ:AAPL"}],"total_items":1}), 1),
        (json!({"data":[]}), 2),
        (json!({"data":[{"last_price":1}],"total_items":1}), 2),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/v3/symbols/quotes", body).await;
        let result = client(&server).quote("NASDAQ:AAPL").await;
        match case {
            0 => assert!(result.unwrap().is_none()),
            1 => assert!(result.unwrap().unwrap().last_price.is_none()),
            _ => assert!(matches!(result, Err(Error::Decode(_)))),
        }
    }
}

#[tokio::test]
async fn quote_adjustments_are_explicit() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/quotes"))
        .and(query_param("settlement", "true"))
        .and(query_param("split", "false"))
        .and(query_param("dadj", "false"))
        .and(query_param("badj", "false"))
        .and(query_param("extended", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(quotes()))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .quotes_with_options(
            &["NASDAQ:AAPL"],
            &QuoteQuery {
                settlement: Some(true),
                split_adjusted: Some(false),
                dividend_adjusted: Some(false),
                back_adjusted: Some(false),
                extended: Some(false),
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn new_and_older_market_status_values_are_retained() {
    for status in ["HOLIDAY", "HOLIDAYS", "HALTED"] {
        let server = MockServer::start().await;
        let mut q = quote();
        q["status"] = json!(status);
        respond(
            &server,
            "/v3/symbols/quotes",
            json!({"data":[q],"total_items":1}),
        )
        .await;
        assert_eq!(
            client(&server)
                .quote("NASDAQ:AAPL")
                .await
                .unwrap()
                .unwrap()
                .status
                .unwrap()
                .as_str(),
            status
        );
    }
}

#[tokio::test]
async fn series_supports_new_adjustments_and_forces_object_rows() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/NASDAQ%3AAAPL/series"))
        .and(query_param("bar_type", "day"))
        .and(query_param("bar_interval", "1"))
        .and(query_param("dp", "5"))
        .and(query_param("24h", "true"))
        .and(query_param("split", "true"))
        .and(query_param("dadj", "true"))
        .and(query_param("badj", "false"))
        .and(query_param("currency", "USD"))
        .and(query_param("settlement", "false"))
        .and(query_param("abbr", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(series()))
        .expect(1)
        .mount(&server)
        .await;
    let data = client(&server)
        .series(
            "NASDAQ:AAPL",
            &SeriesQuery {
                bar_type: Some(GetV3SymbolsSymbolSeriesBarType::Day),
                bar_interval: Some(1),
                data_points: Some(5),
                twenty_four_hour: Some(true),
                split_adjusted: Some(true),
                dividend_adjusted: Some(true),
                back_adjusted: Some(false),
                currency: Some(GetV3SymbolsSymbolSeriesCurrency::Usd),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(data.series[0].close, 123.456);
    assert!(data.series[0].high.is_none());
    assert!(
        !server.received_requests().await.unwrap()[0]
            .url
            .query_pairs()
            .any(|(key, _)| key == "long_poll")
    );
}

#[tokio::test]
async fn history_preserves_archive_metadata_and_month_dates() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/NASDAQ%3AAAPL/history"))
        .and(query_param("start_date","2025-01")).and(query_param("bar_type","hour"))
        .and(query_param("settlement","false")).and(query_param("abbr","false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"code":"NASDAQ:AAPL","bar_type":"1h","series":[],"available_start_date":"2000-01"})))
        .expect(1).mount(&server).await;
    let result = client(&server)
        .history(
            "NASDAQ:AAPL",
            &HistoryQuery::new(GetV3SymbolsSymbolHistoryBarType::Hour, "2025-01"),
        )
        .await
        .unwrap();
    assert!(result.series.is_empty());
    assert_eq!(result.available_start_date.as_deref(), Some("2000-01"));
}

#[tokio::test]
async fn unavailable_archive_is_a_typed_error() {
    let server = MockServer::start().await;
    respond(&server,"/v3/symbols/NASDAQ%3AAAPL/history",json!({"code":"NASDAQ:AAPL","bar_type":"1h","series":[],"error":"no_data","message":"Outside coverage","available_start_date":"2020-01"})).await;
    assert!(
        matches!(client(&server).history("NASDAQ:AAPL",&HistoryQuery::new(GetV3SymbolsSymbolHistoryBarType::Hour,"2000-01")).await,
        Err(Error::HistoryUnavailable {code,message,available_start_date,..}) if code=="no_data" && message.as_deref()==Some("Outside coverage") && available_start_date.as_deref()==Some("2020-01"))
    );
}

#[tokio::test]
async fn missing_close_is_rejected_instead_of_zero() {
    let server = MockServer::start().await;
    let mut body = series();
    body["series"][0].as_object_mut().unwrap().remove("close");
    respond(&server, "/v3/symbols/NASDAQ%3AAAPL/series", body).await;
    assert!(matches!(
        client(&server)
            .series("NASDAQ:AAPL", &SeriesQuery::default())
            .await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn search_sends_exchange_country_and_page_separately() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/search"))
        .and(query_param("query", "Apple & Co"))
        .and(query_param("exchange", "NASDAQ"))
        .and(query_param("country", "US"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"symbols":[],"current_page":2,"has_more":false})),
        )
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .search(&SearchQuery {
            query: Some("Apple & Co".into()),
            exchange: Some("NASDAQ".into()),
            country: Some("US".into()),
            page: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn fundamentals_use_the_new_required_envelope_and_ids() {
    let server = MockServer::start().await;
    respond(&server,"/v3/symbols/NASDAQ%3AAAPL/fundamentals",json!({"code":"NASDAQ:AAPL","last_update":1738022400000i64,"data":[{"id":"price_earnings_fy","value":8.5}]})).await;
    let response = client(&server).fundamentals("NASDAQ:AAPL").await.unwrap();
    assert_eq!(response.data[0].id, "price_earnings_fy");
    let server = MockServer::start().await;
    respond(
        &server,
        "/v3/symbols/NASDAQ%3AAAPL/fundamentals",
        json!({"code":"NASDAQ:AAPL","last_update":1738022400000i64,"data":[{"id":null}]}),
    )
    .await;
    assert!(matches!(
        client(&server).fundamentals("NASDAQ:AAPL").await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn fundamental_series_keeps_named_numeric_lines_without_close() {
    let server = MockServer::start().await;
    respond(&server,"/v3/symbols/NASDAQ%3AAAPL/fundamentals/series",json!({"code":"NASDAQ:AAPL","last_update":1738022400000i64,"total_items":1,"data":[{"id":"rsi","name":"RSI","data":[{"time":1738022400,"rsi":45.5,"plot":44.1}]}]})).await;
    let response = client(&server)
        .fundamentals_series("NASDAQ:AAPL", &["rsi"])
        .await
        .unwrap();
    let point = &response.data[0].data[0];
    assert!(point.close.is_none());
    assert_eq!(point.additional_properties["rsi"], 45.5);
}

#[tokio::test]
async fn document_no_data_is_preserved_by_raw_client() {
    let server = MockServer::start().await;
    Mock::given(path("/v3/documents/test"))
        .and(query_param("code", "NASDAQ:AAPL"))
        .and(query_param("text", "true"))
        .and(header("authorization", "Bearer direct-secret"))
        .and(no_rapid_auth)
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"data":null,"error":"no_data","message":"Document content is unavailable"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let response = client(&server)
        .raw()
        .get_v3_documents_id("test", "NASDAQ:AAPL", Some(true))
        .await
        .unwrap();
    assert!(matches!(
        response,
        GetV3DocumentsIdResponse::UnavailableDocument(_)
    ));
}

#[tokio::test]
async fn info_and_session_preserve_their_metadata() {
    let server = MockServer::start().await;
    respond(
        &server,
        "/v3/symbols/NASDAQ%3AAAPL/info",
        json!({"code":"NASDAQ:AAPL","week_52_high":200.5}),
    )
    .await;
    respond(&server,"/v3/symbols/NASDAQ%3AAAPL/session",json!({"code":"NASDAQ:AAPL","last_update":1738022400000i64,"start_hour":"2100F","end_hour":"2100"})).await;
    assert_eq!(
        client(&server)
            .info("NASDAQ:AAPL")
            .await
            .unwrap()
            .week_52_high,
        Some(200.5)
    );
    assert_eq!(
        client(&server)
            .session("NASDAQ:AAPL")
            .await
            .unwrap()
            .start_hour
            .as_deref(),
        Some("2100F")
    );
}

#[tokio::test]
async fn http_errors_do_not_retry_or_fall_back() {
    for status in [401, 403, 429, 500] {
        let server = MockServer::start().await;
        Mock::given(path("/v3/symbols/quotes"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Retry-After", "60")
                    .set_body_string("provider detail"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server).quotes(&["NASDAQ:AAPL"]).await.unwrap_err();
        assert_eq!(error.is_rate_limited(), status == 429);
        assert!(
            matches!(error,Error::Api{status:actual,body,retry_after} if actual==status && body=="provider detail" && retry_after.as_deref()==Some("60"))
        );
    }
}

#[tokio::test]
async fn buffered_body_limit_and_deadline_are_enforced() {
    let server = MockServer::start().await;
    respond(&server, "/v3/symbols/quotes", quotes()).await;
    assert!(matches!(
        client(&server)
            .with_max_response_body_bytes(8)
            .quotes(&["NASDAQ:AAPL"])
            .await,
        Err(Error::Transport(_))
    ));
    let server = MockServer::start().await;
    Mock::given(path("/v3/symbols/quotes"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(200))
                .set_body_json(quotes()),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server)
            .with_timeout(std::time::Duration::from_millis(50))
            .quotes(&["NASDAQ:AAPL"])
            .await,
        Err(Error::Timeout(_))
    ));
}

#[tokio::test]
async fn invalid_requests_never_reach_the_network() {
    let server = MockServer::start().await;
    let client = client(&server);
    assert!(matches!(
        client.quotes(&[]).await,
        Err(Error::InvalidRequest(_))
    ));
    assert!(matches!(
        client.fundamentals_series("test", &[]).await,
        Err(Error::InvalidRequest(_))
    ));
    assert!(matches!(
        client
            .series(
                "test",
                &SeriesQuery {
                    data_points: Some(0),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::InvalidRequest(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn debug_output_omits_the_token() {
    assert!(!format!("{:?}", Client::new("direct-secret")).contains("direct-secret"));
}
