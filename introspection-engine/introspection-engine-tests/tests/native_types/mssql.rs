use indoc::indoc;
use introspection_engine_tests::test_api::*;
use pretty_assertions::assert_eq;
use test_macros::test_each_connector;

const TYPES: &[(&str, &str)] = &[
    //fieldname, db datatype
    ("int", "Int"),
    ("smallint", "SmallInt"),
    ("tinyint", "TinyInt"),
    ("bigint", "BigInt"),
    ("decimal", "Decimal(5,3)"),
    ("decimal_2", "Decimal"),
    ("numeric", "Numeric(4,1)"),
    ("numeric_2", "Numeric"),
    ("money", "Money"),
    ("smallmoney", "SmallMoney"),
    ("float", "Real"),
    ("double", "Float(53)"),
    ("bit", "Bit"),
    ("chars", "Char(10)"),
    ("nchars", "NChar(10)"),
    ("varchars", "VarChar(500)"),
    ("varchars_2", "VarChar(Max)"),
    ("nvarchars", "NVarChar(500)"),
    ("nvarchars_2", "NVarChar(Max)"),
    ("binary", "Binary(230)"),
    ("varbinary", "VarBinary(150)"),
    ("varbinary_2", "VarBinary(Max)"),
    ("date", "Date"),
    ("time", "Time"),
    ("datetime", "DateTime"),
    ("datetime2", "DateTime2"),
    ("xml", "Xml"),
    ("image", "Image"),
    ("text", "Text"),
    ("ntext", "NText"),
];

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn native_type_columns_feature_on(api: &TestApi) -> crate::TestResult {
    let columns: Vec<String> = TYPES
        .iter()
        .map(|(name, db_type)| format!("[{}] {} NOT NULL", name, db_type))
        .collect();

    api.barrel()
        .execute_with_schema(
            move |migration| {
                migration.create_table("Blog", move |t| {
                    t.inject_custom("id int identity(1,1) primary key");

                    for column in &columns {
                        t.inject_custom(column);
                    }
                });
            },
            api.db_name(),
        )
        .await?;

    let mut dm = String::from(indoc! {r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["nativeTypes"]
        }

        datasource sqlserver {
          provider = "sqlserver"
          url      = "sqlserver://localhost:1433"
        }

    "#});

    let types = indoc! {r#"
        model Blog {
          id          Int      @id @default(autoincrement())
          int         Int
          smallint    Int      @sqlserver.SmallInt
          tinyint     Int      @sqlserver.TinyInt
          bigint      BigInt
          decimal     Decimal  @sqlserver.Decimal(5, 3)
          decimal_2   Decimal  @sqlserver.Decimal(18, 0)
          numeric     Decimal  @sqlserver.Decimal(4, 1)
          numeric_2   Decimal  @sqlserver.Decimal(18, 0)
          money       Float    @sqlserver.Money
          smallmoney  Float    @sqlserver.SmallMoney
          float       Float    @sqlserver.Real
          double      Float
          bit         Boolean
          chars       String   @sqlserver.Char(10)
          nchars      String   @sqlserver.NChar(10)
          varchars    String   @sqlserver.VarChar(500)
          varchars_2  String   @sqlserver.VarChar(Max)
          nvarchars   String   @sqlserver.NVarChar(500)
          nvarchars_2 String   @sqlserver.NVarChar(Max)
          binary      Bytes    @sqlserver.Binary(230)
          varbinary   Bytes    @sqlserver.VarBinary(150)
          varbinary_2 Bytes
          date        DateTime @sqlserver.Date
          time        DateTime @sqlserver.Time
          datetime    DateTime @sqlserver.DateTime
          datetime2   DateTime
          xml         String   @sqlserver.Xml
          image       Bytes    @sqlserver.Image
          text        String   @sqlserver.Text
          ntext       String   @sqlserver.NText
        }
    "#};

    let result = api.re_introspect(&dm).await?;

    dm.push_str(&types);

    println!("EXPECTATION: \n {:#}", dm);
    println!("RESULT: \n {:#}", result);

    assert_eq!(result, dm);

    Ok(())
}
