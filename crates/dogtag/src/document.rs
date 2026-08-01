//! Thin helpers over a spanned generic TOML document.
//!
//! Input parsing deliberately does **not** use `#[derive(Deserialize)]`. The
//! SDK walks [`toml::de::DeTable`] by hand because four things depend on it,
//! and a derive can express none of them: every key *and* value carries a span;
//! a malformed asset yields every diagnostic rather than the first; key legality
//! is scoped to the version the asset declares, which needs two passes; and an
//! unknown key is reported as *our* diagnostic with *our* span.
//!
//! Nothing here raises a diagnostic. Each helper returns a neutral fault, and
//! the caller maps it onto its own area's identifier — which is how the
//! contract parser and the installation parser share this code while keeping
//! `contract.*` and `installation.*` apart.
//!
//! `DeTable` iterates in lexicographic key order, so a sweep over one produces
//! diagnostics in a deterministic order before any sorting.

use core::fmt;
use core::ops::Range;

use toml::Spanned;
use toml::de::{DeArray, DeInteger, DeString, DeTable, DeValue};

/// A TOML value's type, as a schema requires it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    /// A TOML string.
    String,
    /// A TOML integer.
    Integer,
    /// A TOML float.
    Float,
    /// A TOML boolean.
    Boolean,
    /// A TOML datetime.
    Datetime,
    /// A TOML array.
    Array,
    /// A TOML table.
    Table,
}

impl ValueType {
    /// The TOML spelling of the type, matching what the parser reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Datetime => "datetime",
            Self::Array => "array",
            Self::Table => "table",
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A value whose TOML type is not the one the schema requires.
///
/// Neutral by design: the caller turns it into `contract.value-wrong-type` or
/// `installation.value-wrong-type` with its own span mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeMismatch {
    /// The type the schema requires.
    pub expected: ValueType,
    /// The type the value actually has.
    pub found: &'static str,
    /// The byte range of the offending value.
    pub span: Range<usize>,
}

impl TypeMismatch {
    /// The fault stated in terms of the key that carried the value.
    pub fn message(&self, key: &str) -> String {
        format!(
            "`{key}` must be a TOML {} but is a TOML {}",
            self.expected, self.found
        )
    }
}

/// A key the declared version does not define.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownKey {
    /// The key as written.
    pub key: String,
    /// The byte range of the key as written.
    pub span: Range<usize>,
}

/// Parses TOML text into a spanned generic document.
///
/// # Errors
///
/// Returns the parser's error, which carries the span of the syntax fault.
pub fn parse(text: &str) -> Result<Spanned<DeTable<'_>>, toml::de::Error> {
    DeTable::parse(text)
}

/// The value a table declares for `key`, with its span.
pub fn get<'a, 'i>(table: &'a DeTable<'i>, key: &str) -> Option<&'a Spanned<DeValue<'i>>> {
    table.get(key)
}

/// The byte range of `key` as it is written, when the table declares it.
pub fn key_span(table: &DeTable<'_>, key: &str) -> Option<Range<usize>> {
    table.get_key_value(key).map(|(found, _)| found.span())
}

/// Every key in `table` outside `allowed`, in the table's key order.
pub fn unknown_keys(table: &DeTable<'_>, allowed: &[&str]) -> Vec<UnknownKey> {
    table
        .iter()
        .map(|(key, _)| key)
        .filter(|key| !allowed.contains(&key_str(key)))
        .map(|key| UnknownKey {
            key: key_str(key).to_owned(),
            span: key.span(),
        })
        .collect()
}

/// The value as a string.
///
/// # Errors
///
/// Returns a [`TypeMismatch`] when the value is not a TOML string.
pub fn expect_string<'a, 'i>(value: &'a Spanned<DeValue<'i>>) -> Result<&'a str, TypeMismatch> {
    value
        .get_ref()
        .as_str()
        .ok_or_else(|| mismatch(value, ValueType::String))
}

/// The value as a boolean.
///
/// # Errors
///
/// Returns a [`TypeMismatch`] when the value is not a TOML boolean.
pub fn expect_bool(value: &Spanned<DeValue<'_>>) -> Result<bool, TypeMismatch> {
    value
        .get_ref()
        .as_bool()
        .ok_or_else(|| mismatch(value, ValueType::Boolean))
}

/// The value as an integer, undecoded, so a caller can classify a negative or
/// over-large one precisely from its digits and radix.
///
/// # Errors
///
/// Returns a [`TypeMismatch`] when the value is not a TOML integer.
pub fn expect_integer<'a, 'i>(
    value: &'a Spanned<DeValue<'i>>,
) -> Result<&'a DeInteger<'i>, TypeMismatch> {
    value
        .get_ref()
        .as_integer()
        .ok_or_else(|| mismatch(value, ValueType::Integer))
}

/// The value as an array.
///
/// # Errors
///
/// Returns a [`TypeMismatch`] when the value is not a TOML array.
pub fn expect_array<'a, 'i>(
    value: &'a Spanned<DeValue<'i>>,
) -> Result<&'a DeArray<'i>, TypeMismatch> {
    value
        .get_ref()
        .as_array()
        .ok_or_else(|| mismatch(value, ValueType::Array))
}

/// The value as a table.
///
/// # Errors
///
/// Returns a [`TypeMismatch`] when the value is not a TOML table.
pub fn expect_table<'a, 'i>(
    value: &'a Spanned<DeValue<'i>>,
) -> Result<&'a DeTable<'i>, TypeMismatch> {
    value
        .get_ref()
        .as_table()
        .ok_or_else(|| mismatch(value, ValueType::Table))
}

fn mismatch(value: &Spanned<DeValue<'_>>, expected: ValueType) -> TypeMismatch {
    TypeMismatch {
        expected,
        found: value.get_ref().type_str(),
        span: value.span(),
    }
}

fn key_str<'a>(key: &'a Spanned<DeString<'_>>) -> &'a str {
    key.get_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = concat!(
        "contract_version = 1\n",
        "flavour = \"plain\"\n",
        "required = true\n",
        "values = [\"draft\"]\n",
        "\n",
        "[dialect]\n",
        "links = \"wikilink\"\n",
    );

    fn document() -> Spanned<DeTable<'static>> {
        parse(SOURCE).expect("valid TOML")
    }

    #[test]
    fn a_syntax_fault_is_returned_with_its_span() {
        let error = parse("a = =\n").expect_err("malformed");
        assert!(error.span().is_some());
    }

    #[test]
    fn a_key_is_fetched_with_its_span() {
        let document = document();
        let root = document.get_ref();
        assert!(get(root, "contract_version").is_some());
        assert!(get(root, "absent").is_none());
        assert_eq!(key_span(root, "contract_version"), Some(0..16));
        assert_eq!(key_span(root, "absent"), None);
    }

    #[test]
    fn a_value_span_points_at_the_value() {
        let document = document();
        let value = get(document.get_ref(), "contract_version").expect("declared");
        assert_eq!(value.span(), 19..20);
    }

    #[test]
    fn typed_accessors_return_the_value_when_the_type_matches() {
        let document = document();
        let root = document.get_ref();
        let flavour = get(root, "flavour").expect("declared");
        let required = get(root, "required").expect("declared");
        let version = get(root, "contract_version").expect("declared");
        let values = get(root, "values").expect("declared");
        let dialect = get(root, "dialect").expect("declared");
        assert_eq!(expect_string(flavour).expect("a string"), "plain");
        assert!(expect_bool(required).expect("a boolean"));
        assert_eq!(expect_integer(version).expect("an integer").as_str(), "1");
        assert_eq!(expect_array(values).expect("an array").len(), 1);
        assert!(
            expect_table(dialect)
                .expect("a table")
                .contains_key("links")
        );
    }

    #[test]
    fn typed_accessors_report_the_type_they_found() {
        let document = document();
        let root = document.get_ref();
        let version = get(root, "contract_version").expect("declared");
        let flavour = get(root, "flavour").expect("declared");
        let mismatch = expect_string(version).expect_err("an integer");
        assert_eq!(mismatch.expected, ValueType::String);
        assert_eq!(mismatch.found, "integer");
        assert_eq!(mismatch.span, 19..20);
        assert_eq!(
            mismatch.message("contract_version"),
            "`contract_version` must be a TOML string but is a TOML integer"
        );
        assert_eq!(
            expect_bool(flavour).expect_err("a string").expected,
            ValueType::Boolean
        );
        assert_eq!(
            expect_integer(flavour).expect_err("a string").expected,
            ValueType::Integer
        );
        assert_eq!(
            expect_array(flavour).expect_err("a string").expected,
            ValueType::Array
        );
        assert_eq!(
            expect_table(flavour).expect_err("a string").expected,
            ValueType::Table
        );
    }

    #[test]
    fn a_sweep_reports_every_key_outside_the_allowed_set() {
        let document = document();
        let unknown = unknown_keys(document.get_ref(), &["contract_version", "dialect"]);
        let keys: Vec<&str> = unknown.iter().map(|entry| entry.key.as_str()).collect();
        assert_eq!(keys, ["flavour", "required", "values"]);
        assert_eq!(unknown[0].span, 21..28);
        assert!(unknown_keys(document.get_ref(), &[]).len() > keys.len());
    }

    #[test]
    fn a_sweep_of_a_fully_allowed_table_reports_nothing() {
        let document = document();
        let allowed = [
            "contract_version",
            "flavour",
            "required",
            "values",
            "dialect",
        ];
        assert_eq!(unknown_keys(document.get_ref(), &allowed), Vec::new());
    }

    #[test]
    fn value_types_name_themselves() {
        let types = [
            (ValueType::String, "string"),
            (ValueType::Integer, "integer"),
            (ValueType::Float, "float"),
            (ValueType::Boolean, "boolean"),
            (ValueType::Datetime, "datetime"),
            (ValueType::Array, "array"),
            (ValueType::Table, "table"),
        ];
        for (value_type, name) in types {
            assert_eq!(value_type.name(), name);
            assert_eq!(value_type.to_string(), name);
        }
        let copies = types.to_vec();
        assert_eq!(copies.clone(), copies);
        assert!(format!("{:?}", ValueType::Float).contains("Float"));
    }

    #[test]
    fn faults_clone_and_format() {
        let mismatch = TypeMismatch {
            expected: ValueType::Boolean,
            found: "string",
            span: 4..9,
        };
        assert_eq!(mismatch.clone(), mismatch);
        assert!(format!("{mismatch:?}").contains("Boolean"));
        let unknown = UnknownKey {
            key: "requred".to_owned(),
            span: 0..7,
        };
        assert_eq!(unknown.clone(), unknown);
        assert!(format!("{unknown:?}").contains("requred"));
    }
}
