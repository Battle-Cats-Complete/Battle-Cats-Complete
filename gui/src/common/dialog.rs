use std::path::PathBuf;

use rfd::AsyncFileDialog;

pub async fn file(label: &'static str, extensions: &'static [&'static str]) -> Option<PathBuf> {
    AsyncFileDialog::new()
        .add_filter(label, extensions)
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

pub async fn files(label: &'static str, extensions: &'static [&'static str]) -> Vec<PathBuf> {
    AsyncFileDialog::new()
        .add_filter(label, extensions)
        .pick_files()
        .await
        .map(|handles| handles.iter().map(|handle| handle.path().to_path_buf()).collect())
        .unwrap_or_default()
}

pub async fn folder() -> Option<PathBuf> {
    AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|handle| handle.path().to_path_buf())
}
