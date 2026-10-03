# Changelog

## 0.1.0 — Unreleased

- Split the direct API into its own crate using the supplied API 3.0.24 snapshot.
- Generate all 37 operations with bearer authentication and `Client::new(token)`.
- Handle per-symbol quote errors and unavailable archive history explicitly.
- Expose split/24-hour/quote adjustment options and exchange-specific search.
- Default settlement pricing explicitly to false; request object-form history.
- Preserve stricter response fields, named fundamental-series values and document
  unavailability responses.
- Retain Tokio deadlines, bounded buffered responses and tag-based mise releases.
