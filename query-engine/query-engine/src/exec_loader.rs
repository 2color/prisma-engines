use crate::{PrismaError, PrismaResult};
use connection_string::JdbcString;
use connector::Connector;
use std::str::FromStr;

use datamodel::{
    common::provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Datasource,
};
use query_core::executor::{InterpretingExecutor, QueryExecutor};
use std::{collections::HashMap, path::PathBuf};
use url::Url;

#[cfg(feature = "sql")]
use sql_connector::*;

pub async fn load(source: &Datasource) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    match source.active_provider.as_str() {
        #[cfg(feature = "sql")]
        SQLITE_SOURCE_NAME => sqlite(source).await,

        #[cfg(feature = "sql")]
        MYSQL_SOURCE_NAME => mysql(source).await,

        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => postgres(source).await,

        #[cfg(feature = "sql")]
        MSSQL_SOURCE_NAME => {
            if !feature_flags::get().microsoftSqlServer {
                let error = query_core::CoreError::UnsupportedFeatureError(
                    "Microsoft SQL Server (experimental feature, needs to be enabled)".into(),
                );

                return Err(PrismaError::CoreError(error));
            }

            mssql(source).await
        }

        x => Err(PrismaError::ConfigurationError(format!(
            "Unsupported connector type: {}",
            x
        ))),
    }
}

#[cfg(feature = "sql")]
async fn sqlite(source: &Datasource) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQLite connector...");

    let sqlite = Sqlite::from_source(source).await?;
    let path = PathBuf::from(sqlite.file_path());
    let db_name = path.file_stem().unwrap().to_str().unwrap().to_owned(); // Safe due to previous validations.

    trace!("Loaded SQLite connector.");
    Ok((db_name, sql_executor(sqlite, false)))
}

#[cfg(feature = "sql")]
async fn postgres(source: &Datasource) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading Postgres connector...");

    let database_str = &source.url().value;
    let psql = PostgreSql::from_source(source).await?;

    let url = Url::parse(database_str)?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let db_name = params
        .get("schema")
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("public"));

    let force_transactions = params
        .get("pgbouncer")
        .and_then(|flag| flag.parse().ok())
        .unwrap_or(false);

    trace!("Loaded Postgres connector.");
    Ok((db_name, sql_executor(psql, force_transactions)))
}

#[cfg(feature = "sql")]
async fn mysql(source: &Datasource) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading MySQL connector...");

    let mysql = Mysql::from_source(source).await?;
    let database_str = &source.url().value;

    let url = Url::parse(database_str)?;
    let err_str = "No database found in connection string";

    let mut db_name = url
        .path_segments()
        .ok_or_else(|| PrismaError::ConfigurationError(err_str.into()))?;

    let db_name = db_name.next().expect(err_str).to_owned();

    trace!("Loaded MySQL connector.");
    Ok((db_name, sql_executor(mysql, false)))
}

#[cfg(feature = "sql")]
async fn mssql(source: &Datasource) -> PrismaResult<(String, Box<dyn QueryExecutor + Send + Sync>)> {
    trace!("Loading SQL Server connector...");

    let mssql = Mssql::from_source(source).await?;

    let mut conn = JdbcString::from_str(&format!("jdbc:{}", &source.url().value))?;
    let db_name = conn
        .properties_mut()
        .remove("schema")
        .unwrap_or_else(|| String::from("dbo"));

    trace!("Loaded SQL Server connector.");
    Ok((db_name, sql_executor(mssql, false)))
}

#[cfg(feature = "sql")]
fn sql_executor<T>(connector: T, force_transactions: bool) -> Box<dyn QueryExecutor + Send + Sync>
where
    T: Connector + Send + Sync + 'static,
{
    Box::new(InterpretingExecutor::new(connector, force_transactions))
}
