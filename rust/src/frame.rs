use std::collections::HashSet;
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

pub fn clean_names_df(df: DataFrame, case: &str) -> JanitorResult<DataFrame> {
    let names = dataframe_column_names(&df);
    let cleaned = names::make_clean_names(&names, case)?;
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
    }
}
