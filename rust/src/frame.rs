use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use polars::prelude::*;

use crate::names;

#[derive(Debug)]
pub enum JanitorError {
    Value(String),
    Polars(PolarsError),
}

impl JanitorError {
    fn value(message: impl Into<String>) -> Self {
        Self::Value(message.into())
    }
}

impl Display for JanitorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(message) => formatter.write_str(message),
            Self::Polars(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for JanitorError {}

impl From<PolarsError> for JanitorError {
    fn from(value: PolarsError) -> Self {
        Self::Polars(value)
    }
}

impl From<names::NameError> for JanitorError {
    fn from(value: names::NameError) -> Self {
        Self::Value(value.message().to_string())
    }
}

pub type JanitorResult<T> = Result<T, JanitorError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnSchema {
    pub name: String,
    pub dtype: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeaderSearchColumn {
    Name(String),
    Index(usize),
}

pub fn clean_names_df(df: DataFrame, case: &str) -> JanitorResult<DataFrame> {
    let names = dataframe_column_names(&df);
    let cleaned = names::make_clean_names(&names, case)?;
    rename_columns_df(df, cleaned)
}

pub fn row_to_names_df(
    df: DataFrame,
    row_index: usize,
    remove_row: bool,
    remove_rows_above: bool,
    case: &str,
) -> JanitorResult<DataFrame> {
    validate_row_index(&df, row_index)?;
    let row = df.get_row(row_index)?;
    let names = row
        .0
        .iter()
        .map(any_value_to_header_string)
        .collect::<Vec<_>>();
    let cleaned = names::make_clean_names(&names, case)?;
    let df = rename_columns_df(df, cleaned)?;
    drop_header_rows(df, row_index, remove_row, remove_rows_above)
}

pub fn find_header_df(
    df: &DataFrame,
    value: Option<String>,
    column: Option<HeaderSearchColumn>,
) -> JanitorResult<usize> {
    if df.height() == 0 || df.width() == 0 {
        return Err(JanitorError::value("no header row found"));
    }

    match value {
        Some(value) => find_header_by_value(df, value.trim(), column),
        None => find_first_complete_row(df),
    }
}

pub fn frame_schema_from_dataframe(name: String, df: &DataFrame) -> FrameSchema {
    FrameSchema {
        name,
        columns: df
            .columns()
            .iter()
            .map(|column| ColumnSchema {
                name: column.name().as_str().to_string(),
                dtype: polars_dtype_name(column.dtype()),
            })
            .collect(),
    }
}

pub fn compare_df_cols_schemas(
    schemas: &[FrameSchema],
    return_kind: &str,
) -> JanitorResult<DataFrame> {
    validate_compare_return(return_kind)?;
    validate_schemas(schemas)?;

    let columns = compared_column_names(schemas)
        .into_iter()
        .filter(|column| match return_kind {
            "all" => true,
            "match" => schema_column_matches(schemas, column),
            "mismatch" => !schema_column_matches(schemas, column),
            _ => unreachable!("return kind is validated before filtering"),
        })
        .collect::<Vec<_>>();
    let mut output_columns = Vec::with_capacity(schemas.len() + 1);
    output_columns.push(Series::new("column_name".into(), columns.clone()).into_column());

    for schema in schemas {
        let dtype_by_name = schema
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.dtype.as_str()))
            .collect::<HashMap<_, _>>();
        let values = columns
            .iter()
            .map(|column| {
                dtype_by_name
                    .get(column.as_str())
                    .map(|dtype| (*dtype).to_string())
            })
            .collect::<Vec<_>>();
        output_columns.push(Series::new(schema.name.as_str().into(), values).into_column());
    }

    Ok(DataFrame::new(columns.len(), output_columns)?)
}

pub fn compare_df_cols_same_schemas(schemas: &[FrameSchema]) -> JanitorResult<bool> {
    validate_schemas(schemas)?;
    Ok(compared_column_names(schemas)
        .iter()
        .all(|column| schema_column_matches(schemas, column)))
}

fn rename_columns_df(df: DataFrame, cleaned: Vec<String>) -> JanitorResult<DataFrame> {
    let height = df.height();
    let columns = df
        .columns()
        .iter()
        .zip(cleaned)
        .map(|(column, name)| {
            let mut column = column.clone();
            column.rename(name.into());
            column
        })
        .collect::<Vec<_>>();
    Ok(DataFrame::new(height, columns)?)
}

pub fn remove_empty_df(
    df: DataFrame,
    axis: &str,
    subset: Option<Vec<String>>,
) -> JanitorResult<DataFrame> {
    validate_axis(axis)?;
    match axis {
        "rows" => remove_empty_rows_df(df, subset),
        "cols" => remove_empty_cols_df(df, subset),
        "both" => {
            let df = remove_empty_rows_df(df, subset.clone())?;
            remove_empty_cols_df(df, subset)
        }
        _ => unreachable!("axis is validated before matching"),
    }
}

pub fn remove_constant_df(
    df: DataFrame,
    subset: Option<Vec<String>>,
    ignore_nulls: bool,
) -> JanitorResult<DataFrame> {
    let available = dataframe_column_names(&df);
    let columns = selected_columns(&available, subset, "subset")?;
    let mut drop_columns = Vec::new();

    for column in columns {
        let series = df.column(&column)?.as_materialized_series();
        let distinct_count = if ignore_nulls {
            series.drop_nulls().n_unique()?
        } else {
            series.n_unique()?
        };
        if distinct_count <= 1 {
            drop_columns.push(column);
        }
    }

    drop_columns_df(df, &drop_columns)
}

pub fn get_dupes_df(
    df: DataFrame,
    keys: Vec<String>,
    include_count: bool,
    count_name: &str,
) -> JanitorResult<DataFrame> {
    let columns = dataframe_column_names(&df);
    let lazy = get_dupes_lf(df.lazy(), keys, include_count, count_name, columns)?;
    Ok(lazy.collect()?)
}

fn get_dupes_lf(
    lf: LazyFrame,
    keys: Vec<String>,
    include_count: bool,
    count_name: &str,
    columns: Vec<String>,
) -> JanitorResult<LazyFrame> {
    validate_keys(&keys)?;
    validate_key_columns(&columns, &keys)?;
    if include_count && columns.iter().any(|column| column == count_name) {
        return Err(JanitorError::value(format!(
            "count_name already exists in the frame: {count_name:?}"
        )));
    }

    let key_exprs = keys.iter().map(col).collect::<Vec<_>>();
    let with_counts = lf
        .with_columns([len().over(key_exprs).alias(count_name)])
        .filter(col(count_name).gt(lit(1i64)));

    if include_count {
        Ok(with_counts)
    } else {
        Ok(with_counts.select(columns.iter().map(col).collect::<Vec<_>>()))
    }
}

fn remove_empty_rows_df(df: DataFrame, subset: Option<Vec<String>>) -> JanitorResult<DataFrame> {
    let available = dataframe_column_names(&df);
    let columns = selected_columns(&available, subset, "subset")?;
    let Some(all_null) = all_null_expr(&columns) else {
        return Ok(df);
    };
    Ok(df.lazy().filter(all_null.not()).collect()?)
}

fn remove_empty_cols_df(df: DataFrame, subset: Option<Vec<String>>) -> JanitorResult<DataFrame> {
    let available = dataframe_column_names(&df);
    let columns = selected_columns(&available, subset, "subset")?;
    let drop_columns = columns
        .into_iter()
        .filter(|column| {
            df.column(column)
                .map(|series| series.null_count() == df.height())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    drop_columns_df(df, &drop_columns)
}

fn all_null_expr(columns: &[String]) -> Option<Expr> {
    let mut expressions = columns.iter().map(|column| col(column).is_null());
    let first = expressions.next()?;
    Some(expressions.fold(first, |accumulator, expression| accumulator.and(expression)))
}

fn drop_columns_df(df: DataFrame, drop_columns: &[String]) -> JanitorResult<DataFrame> {
    if drop_columns.is_empty() {
        return Ok(df);
    }

    let drop_set = drop_columns
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let height = df.height();
    let columns = df
        .columns()
        .iter()
        .filter(|column| !drop_set.contains(column.name().as_str()))
        .cloned()
        .collect::<Vec<_>>();
    Ok(DataFrame::new(height, columns)?)
}

fn dataframe_column_names(df: &DataFrame) -> Vec<String> {
    df.get_column_names()
        .into_iter()
        .map(|name| name.as_str().to_string())
        .collect()
}

fn validate_row_index(df: &DataFrame, row_index: usize) -> JanitorResult<()> {
    if row_index < df.height() {
        Ok(())
    } else {
        Err(JanitorError::value(format!(
            "row index {row_index} is out of bounds for frame with {} rows",
            df.height()
        )))
    }
}

fn drop_header_rows(
    df: DataFrame,
    row_index: usize,
    remove_row: bool,
    remove_rows_above: bool,
) -> JanitorResult<DataFrame> {
    if !remove_row && !remove_rows_above {
        return Ok(df);
    }

    let height = df.height();
    if remove_rows_above {
        let offset = row_index + usize::from(remove_row);
        return Ok(df.slice(offset as i64, height.saturating_sub(offset)));
    }

    let before = df.slice(0, row_index);
    let after_offset = row_index + 1;
    let after = df.slice(after_offset as i64, height.saturating_sub(after_offset));
    match (before.height(), after.height()) {
        (0, _) => Ok(after),
        (_, 0) => Ok(before),
        _ => Ok(before.vstack(&after)?),
    }
}

fn find_first_complete_row(df: &DataFrame) -> JanitorResult<usize> {
    for row_index in 0..df.height() {
        let row = df.get_row(row_index)?;
        if row.0.iter().all(|value| !any_value_is_missing(value)) {
            return Ok(row_index);
        }
    }
    Err(JanitorError::value("no header row found"))
}

fn find_header_by_value(
    df: &DataFrame,
    value: &str,
    column: Option<HeaderSearchColumn>,
) -> JanitorResult<usize> {
    let column = resolve_header_search_column(df, column)?;
    for row_index in 0..df.height() {
        let cell = column.as_materialized_series().get(row_index)?;
        if !any_value_is_missing(&cell) && any_value_to_header_string(&cell) == value {
            return Ok(row_index);
        }
    }
    Err(JanitorError::value("no header row found"))
}

fn resolve_header_search_column(
    df: &DataFrame,
    column: Option<HeaderSearchColumn>,
) -> JanitorResult<&Column> {
    match column {
        Some(HeaderSearchColumn::Name(name)) => Ok(df.column(&name)?),
        Some(HeaderSearchColumn::Index(index)) => df
            .select_at_idx(index)
            .ok_or_else(|| JanitorError::value(format!("column index {index} is out of bounds"))),
        None => df
            .select_at_idx(0)
            .ok_or_else(|| JanitorError::value("no header row found")),
    }
}

fn any_value_to_header_string(value: &AnyValue<'_>) -> String {
    match value {
        AnyValue::Null => String::new(),
        _ => value.str_value().trim().to_string(),
    }
}

fn any_value_is_missing(value: &AnyValue<'_>) -> bool {
    match value {
        AnyValue::Null => true,
        _ => value.str_value().trim().is_empty(),
    }
}

fn polars_dtype_name(dtype: &DataType) -> String {
    match dtype.to_string().as_str() {
        "bool" => String::from("Boolean"),
        "str" => String::from("String"),
        "i8" => String::from("Int8"),
        "i16" => String::from("Int16"),
        "i32" => String::from("Int32"),
        "i64" => String::from("Int64"),
        "i128" => String::from("Int128"),
        "u8" => String::from("UInt8"),
        "u16" => String::from("UInt16"),
        "u32" => String::from("UInt32"),
        "u64" => String::from("UInt64"),
        "u128" => String::from("UInt128"),
        "f32" => String::from("Float32"),
        "f64" => String::from("Float64"),
        "null" => String::from("Null"),
        other => other.to_string(),
    }
}

fn validate_compare_return(return_kind: &str) -> JanitorResult<()> {
    if matches!(return_kind, "all" | "match" | "mismatch") {
        Ok(())
    } else {
        Err(JanitorError::value(
            "return_ must be one of: 'all', 'match', 'mismatch'",
        ))
    }
}

fn validate_schemas(schemas: &[FrameSchema]) -> JanitorResult<()> {
    if schemas.is_empty() {
        return Err(JanitorError::value(
            "frames must contain at least one frame",
        ));
    }

    let mut names = HashSet::with_capacity(schemas.len());
    for schema in schemas {
        if schema.name == "column_name" {
            return Err(JanitorError::value(
                "frame names must not include 'column_name'",
            ));
        }
        if !names.insert(schema.name.as_str()) {
            return Err(JanitorError::value(format!(
                "frame names must be unique: {:?}",
                schema.name
            )));
        }
    }

    Ok(())
}

fn compared_column_names(schemas: &[FrameSchema]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut columns = Vec::new();
    for schema in schemas {
        for column in &schema.columns {
            if seen.insert(column.name.as_str()) {
                columns.push(column.name.clone());
            }
        }
    }
    columns
}

fn schema_column_matches(schemas: &[FrameSchema], column: &str) -> bool {
    let mut dtype = None;
    for schema in schemas {
        let Some(current) = schema
            .columns
            .iter()
            .find(|candidate| candidate.name == column)
            .map(|candidate| candidate.dtype.as_str())
        else {
            return false;
        };
        match dtype {
            Some(expected) if expected != current => return false,
            Some(_) => {}
            None => dtype = Some(current),
        }
    }
    true
}

fn validate_axis(axis: &str) -> JanitorResult<()> {
    if matches!(axis, "rows" | "cols" | "both") {
        Ok(())
    } else {
        Err(JanitorError::value(
            "axis must be one of: 'rows', 'cols', 'both'",
        ))
    }
}

fn selected_columns(
    available: &[String],
    subset: Option<Vec<String>>,
    label: &str,
) -> JanitorResult<Vec<String>> {
    let selected = match subset {
        Some(columns) => {
            if columns.is_empty() {
                return Err(JanitorError::value(format!(
                    "{label} must contain at least one column"
                )));
            }
            columns
        }
        None => available.to_vec(),
    };

    let missing = selected
        .iter()
        .filter(|column| !available.contains(column))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(JanitorError::value(format!(
            "{label} columns are not present in the frame: {}",
            python_list(&missing)
        )));
    }
    Ok(selected)
}

fn validate_keys(keys: &[String]) -> JanitorResult<()> {
    if keys.is_empty() {
        Err(JanitorError::value("keys must contain at least one column"))
    } else {
        Ok(())
    }
}

fn validate_key_columns(available: &[String], keys: &[String]) -> JanitorResult<()> {
    let missing = keys
        .iter()
        .filter(|key| !available.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(JanitorError::value(format!(
            "keys are not present in the frame: {}",
            python_list(&missing)
        )))
    }
}

fn python_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("'{value}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use polars::df;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn clean_names_renames_dataframe_columns() {
        let df = df!("Customer ID" => [1, 2], "% Complete" => [0.5, 1.0]).unwrap();

        let result = clean_names_df(df, "snake").unwrap();

        assert_eq!(
            dataframe_column_names(&result),
            ["customer_id", "percent_complete"]
        );
    }

    #[test]
    fn remove_empty_handles_rows_columns_and_both() {
        let df = df!(
            "a" => [None, None, Some(1)],
            "b" => [None::<i32>, None, None],
            "c" => [Some("x"), None, Some("z")]
        )
        .unwrap();

        assert_eq!(
            remove_empty_df(df.clone(), "rows", None).unwrap().height(),
            2
        );
        assert_eq!(
            dataframe_column_names(&remove_empty_df(df.clone(), "cols", None).unwrap()),
            ["a", "c"]
        );
        assert_eq!(
            dataframe_column_names(&remove_empty_df(df, "both", None).unwrap()),
            ["a", "c"]
        );
    }

    #[test]
    fn remove_constant_respects_null_semantics() {
        let df = df!(
            "constant" => [1, 1, 1],
            "with_null" => [Some(1), None, Some(1)],
            "varied" => [1, 2, 1],
            "nulls" => [None::<i32>, None, None]
        )
        .unwrap();

        assert_eq!(
            dataframe_column_names(&remove_constant_df(df.clone(), None, false).unwrap()),
            ["with_null", "varied"]
        );
        assert_eq!(
            dataframe_column_names(&remove_constant_df(df, None, true).unwrap()),
            ["varied"]
        );
    }

    #[test]
    fn get_dupes_returns_only_duplicate_key_rows() {
        let df =
            df!("id" => [1, 1, 2, 3, 3, 3], "value" => ["a", "b", "c", "d", "e", "f"]).unwrap();

        let result = get_dupes_df(df, vec![String::from("id")], true, "duplicate_count").unwrap();

        assert_eq!(result.height(), 5);
        assert!(dataframe_column_names(&result).contains(&String::from("duplicate_count")));
    }

    #[test]
    fn find_header_and_row_to_names_handle_messy_spreadsheets() {
        let df = df!(
            "column_1" => [None, Some("Customer ID"), Some("1"), Some("2")],
            "column_2" => [Some("notes"), Some("Order Total"), Some("10"), Some("20")]
        )
        .unwrap();

        let header = find_header_df(&df, None, None).unwrap();
        let result = row_to_names_df(df, header, true, true, "snake").unwrap();

        assert_eq!(header, 1);
        assert_eq!(
            dataframe_column_names(&result),
            ["customer_id", "order_total"]
        );
        assert_eq!(result.height(), 2);
    }

    #[test]
    fn find_header_can_search_a_specific_column() {
        let df = df!(
            "left" => [None, Some("ignore"), Some("wrong")],
            "right" => [Some("skip"), Some("Header"), Some("Header")]
        )
        .unwrap();

        assert_eq!(
            find_header_df(
                &df,
                Some(String::from("Header")),
                Some(HeaderSearchColumn::Name(String::from("right")))
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn compare_df_cols_reports_matches_and_mismatches() {
        let left = FrameSchema {
            name: String::from("left"),
            columns: vec![
                ColumnSchema {
                    name: String::from("id"),
                    dtype: String::from("Int64"),
                },
                ColumnSchema {
                    name: String::from("name"),
                    dtype: String::from("String"),
                },
            ],
        };
        let right = FrameSchema {
            name: String::from("right"),
            columns: vec![
                ColumnSchema {
                    name: String::from("id"),
                    dtype: String::from("Int64"),
                },
                ColumnSchema {
                    name: String::from("name"),
                    dtype: String::from("Categorical"),
                },
            ],
        };

        let result = compare_df_cols_schemas(&[left.clone(), right.clone()], "mismatch").unwrap();

        assert!(!compare_df_cols_same_schemas(&[left, right]).unwrap());
        assert_eq!(result.height(), 1);
    }

    proptest! {
        #[test]
        fn get_dupes_only_returns_rows_from_duplicate_key_groups(
            keys in proptest::collection::vec(any::<i64>(), 0..128)
        ) {
            let mut counts = HashMap::new();
            for key in &keys {
                *counts.entry(*key).or_insert(0u32) += 1;
            }

            let rows = (0..keys.len() as i64).collect::<Vec<_>>();
            let df = df!("key" => keys.clone(), "row" => rows).unwrap();
            let result = get_dupes_df(df, vec![String::from("key")], true, "duplicate_count").unwrap();
            let expected_height = keys.iter().filter(|key| counts[key] > 1).count();

            prop_assert_eq!(result.height(), expected_height);

            let result_keys = result
                .column("key")
                .unwrap()
                .as_materialized_series()
                .i64()
                .unwrap();
            let duplicate_counts = result
                .column("duplicate_count")
                .unwrap()
                .as_materialized_series()
                .u32()
                .unwrap();

            for (key, duplicate_count) in result_keys.iter().zip(duplicate_counts.iter()) {
                prop_assert!(key.is_some(), "result key should not be null");
                prop_assert!(
                    duplicate_count.is_some(),
                    "duplicate_count should not be null"
                );

                let key = key.unwrap();
                let duplicate_count = duplicate_count.unwrap();
                prop_assert_eq!(counts[&key], duplicate_count);
                prop_assert!(duplicate_count > 1);
            }
        }

        #[test]
        fn compare_df_cols_same_accepts_identical_generated_schemas(
            columns in proptest::collection::btree_map(
                0u8..64,
                prop_oneof![
                    Just(String::from("Int64")),
                    Just(String::from("String")),
                    Just(String::from("Boolean")),
                    Just(String::from("Float64")),
                ],
                0..64
            )
        ) {
            let columns = columns
                .into_iter()
                .map(|(index, dtype)| ColumnSchema {
                    name: format!("column_{index}"),
                    dtype,
                })
                .collect::<Vec<_>>();
            let column_count = columns.len();
            let schema = FrameSchema {
                name: String::from("left"),
                columns,
            };
            let same_schema = FrameSchema {
                name: String::from("right"),
                columns: schema.columns.clone(),
            };

            prop_assert!(compare_df_cols_same_schemas(&[schema.clone(), same_schema.clone()]).unwrap());
            prop_assert_eq!(
                compare_df_cols_schemas(&[schema, same_schema], "all").unwrap().height(),
                column_count
            );
        }
    }
}
