use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

struct Commit {
    tree_hash: String,
    parent_hashes: Vec<String>,
    message: String,
}

struct TreeEntry {
    mode: String,
    name: String,
    hash: String,
}

fn read_object(git_dir: &Path, hash: &str) -> Vec<u8> {
    let object_path = git_dir.join("objects").join(&hash[..2]).join(&hash[2..]);
    let compressed =
        fs::read(&object_path).unwrap_or_else(|_| panic!("Failed to read object: {}", hash));
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .unwrap_or_else(|_| panic!("Failed to decompress object: {}", hash));
    decompressed
}

fn parse_object(data: &[u8]) -> (&str, &[u8]) {
    let header_end = data
        .iter()
        .position(|&b| b == 0)
        .expect("Invalid object: no null byte");
    let header =
        std::str::from_utf8(&data[..header_end]).expect("Invalid object: header not valid UTF-8");
    let content = &data[header_end + 1..];
    let mut parts = header.splitn(2, ' ');
    let obj_type = parts.next().unwrap();
    let _size = parts.next().unwrap();
    (obj_type, content)
}

fn parse_commit(content: &[u8]) -> Commit {
    let content_str = std::str::from_utf8(content).expect("Invalid commit: not valid UTF-8");
    let mut tree_hash = String::new();
    let mut parent_hashes = Vec::new();
    let mut message = String::new();
    let mut in_message = false;
    for line in content_str.lines() {
        if in_message {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(line);
            continue;
        }
        if line.is_empty() {
            in_message = true;
            continue;
        }
        if let Some(hash) = line.strip_prefix("tree ") {
            tree_hash = hash.to_string();
        } else if let Some(hash) = line.strip_prefix("parent ") {
            parent_hashes.push(hash.to_string());
        }
    }
    Commit {
        tree_hash,
        parent_hashes,
        message,
    }
}

fn read_commit(git_dir: &Path, hash: &str, cache: &mut HashMap<String, Commit>) {
    if cache.contains_key(hash) {
        return;
    }
    let data = read_object(git_dir, hash);
    let (obj_type, content) = parse_object(&data);
    match obj_type {
        "commit" => {
            let commit = parse_commit(content);
            for parent in &commit.parent_hashes {
                read_commit(git_dir, parent, cache);
            }
            cache.insert(hash.to_string(), commit);
        }
        _ => {
            panic!("Expected commit object, got {}", obj_type);
        }
    }
}

fn log_oneline(commits: &HashMap<String, Commit>, start_hash: &str) {
    let mut hash = start_hash.to_string();
    loop {
        match commits.get(&hash) {
            Some(commit) => {
                let short_hash = &hash[..7];
                let first_line = commit.message.lines().next().unwrap_or("");
                println!("{} {}", short_hash, first_line);
                match commit.parent_hashes.first() {
                    Some(parent) => hash = parent.clone(),
                    None => break,
                }
            }
            None => break,
        }
    }
}

fn parse_tree(content: &[u8]) -> Vec<TreeEntry> {
    let mut entries = Vec::new();
    let mut pos = 0;
    while pos < content.len() {
        let null_pos = content[pos..]
            .iter()
            .position(|&b| b == 0)
            .expect("Invalid tree: missing null byte")
            + pos;
        let mode_and_name =
            std::str::from_utf8(&content[pos..null_pos]).expect("Invalid tree: not valid UTF-8");
        let space_pos = mode_and_name
            .find(' ')
            .expect("Invalid tree entry: missing space");
        let mode = &mode_and_name[..space_pos];
        let name = &mode_and_name[space_pos + 1..];
        pos = null_pos + 1;
        let hash_bytes = &content[pos..pos + 20];
        let hash = hash_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        entries.push(TreeEntry {
            mode: mode.to_string(),
            name: name.to_string(),
            hash,
        });
        pos += 20;
    }
    entries
}

fn walk_tree(git_dir: &Path, tree_hash: &str, prefix: &str) -> Vec<String> {
    let data = read_object(git_dir, tree_hash);
    let (_, content) = parse_object(&data);
    let entries = parse_tree(content);
    entries
        .iter()
        .flat_map(|entry| {
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", prefix, entry.name)
            };
            if entry.mode == "040000" {
                walk_tree(git_dir, &entry.hash, &path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn read_head(git_dir: &Path) -> String {
    let head_path = git_dir.join("HEAD");
    let content = fs::read_to_string(&head_path).expect("Failed to read HEAD");
    content.trim().to_string()
}

fn resolve_ref(git_dir: &Path, reference: &str) -> Option<String> {
    if reference.starts_with("ref: ") {
        let ref_path = &reference[5..];
        let full_path = git_dir.join(ref_path);
        fs::read_to_string(&full_path)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        Some(reference.to_string())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let git_dir = if args.len() > 1 {
        Path::new(&args[1]).to_path_buf()
    } else {
        Path::new(".git").to_path_buf()
    };
    let head = read_head(&git_dir);
    let head_hash = resolve_ref(&git_dir, &head).expect("Failed to resolve HEAD");
    let mut cache = HashMap::new();
    read_commit(&git_dir, &head_hash, &mut cache);
    log_oneline(&cache, &head_hash);
    if let Some(commit) = cache.get(&head_hash) {
        println!("\nFiles in latest commit:");
        let files = walk_tree(&git_dir, &commit.tree_hash, "");
        for file in &files {
            println!("  {}", file);
        }
    }
}
