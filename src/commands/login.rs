use crate::api::endpoints::{exchange_device_code_for_token, initiate_device_code};
use crate::auth::callback_server;
use crate::auth::AuthService;
use crate::errors::AppError;
use tokio::sync::oneshot;

/// Starts the OAuth device flow and saves the token.
pub async fn execute(auth_service: &mut AuthService, client_id: &str) -> Result<(), AppError> {
    // spawn callback server
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Err(e) = callback_server::start_callback_server(tx).await {
            eprintln!("Callback server error: {e}");
        }
    });

    // get device code
    let resp = initiate_device_code(auth_service.api_client(), client_id)
        .await
        .map_err(AppError::Api)?;
    println!(
        "\nVisit {} and enter code {} then press Enter...",
        resp.verification_uri, resp.user_code
    );
    let _ = std::io::stdin().read_line(&mut String::new());

    // open browser
    let _ = webbrowser::open(&resp.verification_uri_complete);

    // wait for callback
    let code = rx.await.map_err(|_| AppError::Callback)?;

    // exchange for token
    let tok = exchange_device_code_for_token(auth_service.api_client(), &code)
        .await
        .map_err(AppError::Api)?;
    auth_service.save_access_token(&tok.access_token)?;

    println!("Authentication successful!");
    Ok(())
}
