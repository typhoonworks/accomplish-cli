use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

/// Reads the token file if it exists, returning Ok(Some(token)) or Ok(None).
pub fn load_token(path: &PathBuf) -> io::Result<Option<String>> {
    if path.exists() {
        let token = fs::read_to_string(path)?.trim().to_string();
        Ok(Some(token))
    } else {
        Ok(None)
    }
}

/// Writes `token` to the file, creating parent dirs and setting 0o600 perms on Unix.
pub fn save_token(path: &PathBuf, token: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = fs::File::create(path)?;
    file.write_all(token.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Deletes the token file if it exists.
pub fn clear_token(path: &PathBuf) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
