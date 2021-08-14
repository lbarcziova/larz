use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::convert::TryInto;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Bytes;
use std::io::Read;
use std::io::Write;
use std::option::Option;
use std::path::Path;
use std::path::PathBuf;

/// Archive a file or set of files
///
/// # Arguments
///
/// * `PATH` - Path to a file or directory (required)
pub fn compress_archive(matches: &clap::ArgMatches) {
    let paths: Vec<&str> = matches
        .values_of("PATH")
        .with_context(|| "No file paths were given".to_string())
        .unwrap()
        .collect();
    let output_path = matches
        .value_of("out")
        .with_context(|| "No output path was given".to_string())
        .unwrap();

    let output_pathbuf = PathBuf::from(output_path);
    let output_file_name = output_pathbuf.file_stem().unwrap().to_str().unwrap();

    let mut tar = tar::Builder::new(Vec::new());
    
    for fs_path in paths {
        let path_buf = PathBuf::from(fs_path);
        match path_buf.is_dir()
        {
            true => {
                tar.append_dir_all(".", fs_path)
                .expect("Failed to write to archive");
            }
            false => {
                tar.append_path(fs_path).expect("Failed to write to archive");
            }
        }
    }

    tar.finish().expect("Unable to finish writing archive");
    let tar_bytes: &Vec<u8> = tar.get_ref();
    let compressed_archive_file = lz4_flex::compress_prepend_size(tar_bytes);
    write_file(&output_path, &compressed_archive_file);

    let uncompressed_size_bytes: [u8; 4] = compressed_archive_file[0..4].try_into().unwrap();
    let uncompressed_size: u32 = u32::from_le_bytes(uncompressed_size_bytes);
    println!(
        "Wrote package '{}' to filesystem (path: {}, uncompressed size: {} bytes)",
        output_file_name, output_path, uncompressed_size
    );
}

/// Decompress an archive
///
/// # Arguments
///
/// * `PATH` - Path to an archive (required)
pub fn extract_archive(matches: &clap::ArgMatches) {
    let paths: Vec<&str> = matches
        .values_of("PATH")
        .with_context(|| "No file paths were given".to_string())
        .unwrap()
        .collect();
    let output_path = matches
        .value_of("out")
        .with_context(|| "No output path was given".to_string())
        .unwrap();

    for file_path in paths {
        let compressed_file_bytes = fs::read(file_path).expect("Could not read archive file");
        let decompressed_package = lz4_flex::decompress_size_prepended(&compressed_file_bytes)
            .expect("Could not decompress archive file");
        let decompressed_package_bytes = &decompressed_package[..];
        let mut extracted_package = tar::Archive::new(decompressed_package_bytes);
        extracted_package
            .unpack(&output_path)
            .expect("Could not extract archive");
    
        let file_pathbuf = PathBuf::from(output_path);
        let file_name = file_pathbuf.file_stem().unwrap().to_str().unwrap();
        println!(
            "Extracted archive '{}' to filesystem (path: {})",
            file_name,
            output_path
        );
    }
}

/// Write a file to the filesystem
///
/// # Arguments
///
/// * `path` - The path to write the file to
///
/// * `data_to_write` - The data to write to the filesystem
#[inline(always)]
pub fn write_file(path: &str, data_to_write: &[u8]) {
    fs::create_dir_all(Path::new(path).parent().unwrap()).unwrap(); // Create output path, write to file
    let file = File::create(&path).unwrap(); // Create file which we will write to
    let mut buffered_writer = BufWriter::new(file); // Create a buffered writer, allowing us to modify the file we've just created
    buffered_writer
        .write_all(data_to_write)
        .with_context(|| format!("Could not write data to {}", path))
        .unwrap(); // Write data to file
    buffered_writer.flush().unwrap(); // Empty out the data from memory after we've written to the file
}
