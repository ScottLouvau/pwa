use std::{env, fs::{self, OpenOptions}, io::{self, BufRead, BufReader, Write}, path::PathBuf};
use sha2::{Digest, Sha256};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: hash-files <pathToIndex> <outputServiceJsPath>");
        println!(" Computes the SHA-256 hash of all file bytes under <pathToIndex> except the first line of <outputServiceJsPath>.");
        println!(" Replaces the first line of <outputServiceJsPath> with 'const CACHE_VERSION = \"<Hash>\";'");
        return;
    }
    
    let input_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);

    let mut hasher: Sha256 = Sha256::new();
    hash_files_recursively(&input_path, &output_path, &mut hasher).unwrap();
    let hash = hasher.finalize();
    let mut hash = hex::encode(hash);
    hash.truncate(8);
    
    let new_first_line = format!("const CACHE_VERSION = \"{hash}\";");
    replace_file_first_line(&output_path, &new_first_line).unwrap();

    println!("Wrote:\n {new_first_line}\nto:\n {output_path:?}");
}

fn hash_files_recursively(path: &PathBuf, except_path: &PathBuf, hasher: &mut Sha256) -> io::Result<()> {
    let paths = std::fs::read_dir(path)?;
    for path in paths {
        let path = path?.path();
        if path.is_dir() {
            hash_files_recursively(&path, except_path, hasher)?;
        } else {
            add_file_to_hash(&path, except_path, hasher)?;
        }
    }

    Ok(())
}

fn add_file_to_hash(path: &PathBuf, except_path: &PathBuf, hasher: &mut Sha256) -> io::Result<()> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    // If this is the output file we will put the hash on the first line of, skip the first line of it,
    // so that multiple runs compute the same hash for otherwise-identical contents.
    if path.eq(except_path) {
        let mut unused = String::new();
        reader.read_line(&mut unused)?;
    }
    
    // Add the contents of the file to the hash
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

fn replace_file_first_line(path: &PathBuf, new_first_line: &str) -> io::Result<()> {
    let contents = fs::read_to_string(path)?;
    let lines = contents.lines().skip(1);

    let mut file = OpenOptions::new().write(true).open(path)?;
    file.write_all(new_first_line.as_bytes())?;
    file.write(b"\n")?;

    for line in lines {
        file.write_all(line.as_bytes())?;
        file.write(b"\n")?;
    }

    Ok(())
}