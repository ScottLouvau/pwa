use std::{env, fs, io::{self, BufRead, BufReader}, path::PathBuf};
use sha2::{Digest, Sha256};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: hash-files <pathToIndex>");
        return;
    }
    
    let input_path = PathBuf::from(&args[1]);
    let mut hasher: Sha256 = Sha256::new();
    hash_files_recursively(&input_path, &mut hasher).unwrap();

    let hash = hasher.finalize();
    let hash = hex::encode(hash);

    println!("Hash of {input_path:?} is {hash}");    
}

fn hash_files_recursively(path: &PathBuf, hasher: &mut Sha256) -> io::Result<()> {
    let paths = std::fs::read_dir(path)?;
    for path in paths {
        let path = path?.path();
        if path.is_dir() {
            hash_files_recursively(&path, hasher)?;
        } else {
            add_file_to_hash(&path, hasher)?;
        }
    }

    Ok(())
}

fn add_file_to_hash(path: &PathBuf, hasher: &mut Sha256) -> io::Result<()> {
    let file = fs::File::open(path).unwrap();
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    loop {
        let buffer = reader.fill_buf()?;
        let length = buffer.len();

        if length == 0 {
            break;
        }

        hasher.update(buffer);
        reader.consume(length);
    }

    Ok(())
}