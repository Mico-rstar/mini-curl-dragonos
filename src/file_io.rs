use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

/// 字符串覆盖写入
pub fn write_string_to_file(path: &str, contents: &str) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())
}

/// 字符串追加写入
pub fn append_string_to_file(path: &str, contents: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(contents.as_bytes())
}

/// 二进制覆盖写入
pub fn write_bytes_to_file(path: &str, data: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(data)
}

/// 二进制读取
pub fn read_file_to_bytes(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// 字符串读取
pub fn read_file_to_string(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}