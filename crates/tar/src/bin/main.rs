use std::fs::read;

use tar::Archive;

fn main() {
    let bytes = read("bootstrap.tar").expect("Can't find file");

    let archive = Archive(bytes.as_slice());

    for file in archive.files() {
        println!("Found file {} with size {:?} B", file.header_record.path(), file.header_record.size())
    }
}
