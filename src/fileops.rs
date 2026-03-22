use std::path::PathBuf;

pub async fn read_file(root_dir: &str, file_name: &str) -> std::io::Result<Vec<u8>> {
  let path = PathBuf::from(root_dir).join(file_name);
  tokio::fs::read(path).await
}
