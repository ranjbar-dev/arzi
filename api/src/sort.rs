/// Builds a safe `ORDER BY` clause from a client-supplied `sort`/`order` pair.
/// `allowed` whitelists which client-facing sort keys map to which SQL
/// column expressions — `sort` is never interpolated into SQL directly, so
/// there is no injection surface even though this returns a raw string
/// spliced into the query (identifiers can't be bind parameters).
pub fn order_by(
    sort: Option<&str>,
    order: Option<&str>,
    allowed: &[(&str, &str)],
    default: &str,
) -> String {
    let dir = if order == Some("desc") { "DESC" } else { "ASC" };
    match sort.and_then(|s| allowed.iter().find(|(key, _)| *key == s)) {
        Some((_, column)) => format!("{column} {dir}"),
        None => default.to_string(),
    }
}
