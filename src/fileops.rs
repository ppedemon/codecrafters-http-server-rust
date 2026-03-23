use std::io;
use std::path::PathBuf;

pub async fn read_file(root_dir: &str, file_name: &str) -> io::Result<Vec<u8>> {
    let path = PathBuf::from(root_dir).join(file_name);
    tokio::fs::read(path).await
}

pub async fn write_file(root_dir: &str, file_name: &str, contents: &[u8]) -> io::Result<()> {
    let path = PathBuf::from(root_dir).join(file_name);
    tokio::fs::write(path, contents).await
}
