# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic
Versioning].

<!-- #release:next-header -->

## [Unreleased] <!-- #release:date -->

## [0.9.0] - 2024-01-17

* Support `additional_emails` field on `Customer`.

## [0.8.1] - 2024-01-12

* Fix bug causing deserialization errors when `get_customer_costs` responses
  contained null price dimension values.

## [0.8.0] - 2024-01-05

* Add `base_plan_id` to `Plan`.

## [0.7.2] - 2024-01-03

* Export the Orb `ApiError` struct.

## [0.7.1] - 2023-12-19

* Fix bug preventing customer costs from being filtered by `timeframe_start`.

## [0.7.0] - 2023-12-18

* Add support for Plan metadata.
* Add support for creating customer ledger entries.
* Add support for enumerating customer credit balances.
* Add support for fetching customer costs.
* Add support for filtering by timeframe in `Client::search_events`.
* Bump MSRV to 1.70.

## [0.6.0] - 2023-05-12

* Modified the `InvoiceSubscription` field to be optional.

## [0.5.0] - 2023-05-11

* Add `amount_due` as part of `Invoice`.

## [0.4.0] - 2023-05-09

* Add support for non-default status filters in `Client::list_invoices`.

* Return `hosted_invoice_url` as part of `Invoice`, when available.

* Fix bug causing invoices without `invoice_pdf` set to cause errors.

## [0.3.0] - 2023-04-13

* Add support for request idempotency keys to `Client::create_customer` and
  `Client::create_subscription`.

## [0.2.0] - 2023-03-07

* Uniformly derive `Serialize` and `Deserialize` on all API types, even if the
  type is not serialized or deserialized by `Client`. The idea is to allow
  downstream users to serialize and deserialize these types for their own
  purposes (e.g., to store them on disk).

## 0.1.0 - 2023-01-08

Initial release.

<!-- #release:next-url -->
[Unreleased]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MaterializeInc/rust-orb-billing/compare/v0.1.0...v0.2.0

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
