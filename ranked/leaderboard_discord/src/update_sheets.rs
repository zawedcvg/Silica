use calamine::{open_workbook, Reader, Xlsx};
use google_sheets4::{hyper, hyper_rustls, oauth2, Sheets};
use serde_json::json;
use serde_json::value::Value;
use std::error::Error;

pub async fn update_google_sheet_from_excel(
    excel_path: &str,
    spreadsheet_id: &str,
    sheet_name: &str,
    credentials_path: &str,
) -> Result<(), Box<dyn Error>> {
    // Read the Excel file
    let mut workbook: Xlsx<_> = open_workbook(excel_path)?;
    let range = match workbook.worksheet_range(sheet_name) {
        Ok(range) => range,
        Err(_) => panic!("Couldnt find the file"),
    };

    let mut data = vec![];
    for row in range.rows() {
        let row_data: Vec<Value> = row.iter().map(|cell| json!(cell.to_string())).collect();
        data.push(row_data);
    }

    // Read the credentials file
    let secret = oauth2::read_application_secret(credentials_path).await?;

    // Set up authentication and Sheets API
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokencache.json")
    .build()
    .await?;

    let scopes = &["https://www.googleapis.com/auth/spreadsheets"];
    let _token = auth.token(scopes).await?;

    let hub = Sheets::new(
        hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()?
                .https_or_http()
                .enable_http1()
                .build(),
        ),
        auth,
    );
    let values: Vec<Vec<Value>> = data;
    let value_range = google_sheets4::api::ValueRange {
        range: Some(sheet_name.to_string()),
        values: Some(values),
        ..Default::default()
    };

    // Update the Google Sheet
    hub.spreadsheets()
        .values_update(value_range, spreadsheet_id, sheet_name)
        .value_input_option("RAW")
        .doit()
        .await?;

    Ok(())
}
