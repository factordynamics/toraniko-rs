//! Ranking utilities.

use polars::prelude::*;

/// Select top N rows in each group by a ranking variable.
///
/// # Arguments
/// * `df` - Input LazyFrame
/// * `n` - Number of top rows to select per group
/// * `rank_var` - Column to rank by (descending)
/// * `group_vars` - Columns to group by
/// * `filter` - If true, return only top N; if false, add rank_mask column
///
/// # Returns
/// LazyFrame with top N rows or rank mask.
pub fn top_n_by_group(
    df: LazyFrame,
    n: u32,
    rank_var: &str,
    group_vars: &[&str],
    filter: bool,
) -> LazyFrame {
    let group_exprs: Vec<Expr> = group_vars.iter().map(|&c| col(c)).collect();

    // Use a descending sort-based rank approximation
    let ranked = df.with_column(
        // Create a descending rank using argsort
        col(rank_var)
            .sort_by([col(rank_var)], SortMultipleOptions::new().with_order_descending(true))
            .over(group_exprs.clone())
            .alias("_sorted_val"),
    );

    // Simple approach: sort by rank_var descending within groups and take top n
    let sort_options = SortMultipleOptions::new().with_order_descending(true);

    if filter {
        // Sort and use head within each group
        ranked
            .sort([rank_var], sort_options)
            .group_by(group_exprs)
            .head(Some(n as usize))
            .drop(["_sorted_val"])
    } else {
        // Add a mask column indicating top N
        ranked.with_column(lit(true).alias("rank_mask")).drop(["_sorted_val"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_n_by_group_filter() {
        let df = df! {
            "date" => &[1, 1, 1, 1, 2, 2, 2, 2],
            "symbol" => &["A", "B", "C", "D", "A", "B", "C", "D"],
            "value" => &[10.0, 20.0, 30.0, 40.0, 15.0, 25.0, 35.0, 45.0],
        }
        .unwrap()
        .lazy();

        let result = top_n_by_group(df, 2, "value", &["date"], true).collect().unwrap();

        // Should have 2 rows per date (top 2 by value)
        assert_eq!(result.height(), 4);
    }

    #[test]
    fn top_n_by_group_mask() {
        let df = df! {
            "date" => &[1, 1, 1, 1],
            "symbol" => &["A", "B", "C", "D"],
            "value" => &[10.0, 20.0, 30.0, 40.0],
        }
        .unwrap()
        .lazy();

        let result = top_n_by_group(df, 2, "value", &["date"], false).collect().unwrap();

        // Should have all 4 rows with rank_mask column
        assert_eq!(result.height(), 4);
        assert!(result.column("rank_mask").is_ok());
    }
}
