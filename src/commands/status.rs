use crate::auth::AuthService;
use crate::errors::AppError;

pub async fn execute(auth_service: &mut AuthService) -> Result<(), AppError> {
    match auth_service.ensure_authenticated().await {
        Ok(()) => {
            println!();
            println!("You’re logged in.");
        }
        Err(_) => {
            println!();
            println!("You are not authenticated. Run `accomplish login` first.");
        }
    }
    Ok(())
}
