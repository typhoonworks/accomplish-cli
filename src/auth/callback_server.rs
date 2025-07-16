use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{oneshot, Mutex};

#[derive(Deserialize)]
struct CallbackParams {
    device_code: String,
}

pub async fn start_callback_server(
    tx: oneshot::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Wrap the Sender in an Arc<Mutex<Option<Sender>>> for safe sharing and ownership transfer
    let shared_tx = Arc::new(Mutex::new(Some(tx)));

    let app = Router::new().route(
        "/callback",
        get({
            let shared_tx = Arc::clone(&shared_tx);
            move |Query(params): Query<CallbackParams>| handle_callback(params, shared_tx)
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    // println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_callback(
    params: CallbackParams,
    shared_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
) -> impl IntoResponse {
    if let Some(tx) = shared_tx.lock().await.take() {
        let _ = tx.send(params.device_code.clone());
    }

    (
        StatusCode::OK,
        Html(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Success: GitHub CLI</title>
                <style type="text/css">
                    body {
                        color: #1B1F23;
                        background: #F6F8FA;
                        font-size: 14px;
                        font-family: -apple-system, "Segoe UI", Helvetica, Arial, sans-serif;
                        line-height: 1.5;
                        max-width: 620px;
                        margin: 28px auto;
                        text-align: center;
                    }

                    h1 {
                        font-size: 24px;
                        margin-bottom: 0;
                    }

                    p {
                        margin-top: 0;
                    }

                    .box {
                        border: 1px solid #E1E4E8;
                        background: white;
                        padding: 24px;
                        margin: 28px;
                    }
                </style>
            </head>
            <body>
                <div class="box">
                  <h1>Successfully authenticated Accomplish CLI</h1>
                  <p>You may now close this tab and return to the terminal.</p>
                </div>
            </body>
            </html>
            "#,
        ),
    )
}
