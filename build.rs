use std::{collections::HashMap, path::PathBuf};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(out_dir).join("migrations.rs");

    let mut migrations: HashMap<String, String> = HashMap::new();
    let mut entries: Vec<_> = std::fs::read_dir("./migrations")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap()
                .split(".")
                .next()
                .unwrap();
            let migration = std::fs::read_to_string(&path).unwrap();

            migrations.insert(name.to_string(), migration);
        }
    }

    let mut output = String::from(
        "use std::collections::HashMap;\n\npub(crate) static MIGRATIONS: &[( &str, &str )] = &[\n",
    );

    for (key, value) in &migrations {
        output.push_str(&format!("    (\"{}\", r#\"{}\"#),\n", key, value));
    }

    output.push_str("];\n\npub(crate) fn get_migrations() -> HashMap<String, String> {\n");
    output.push_str(
        "    MIGRATIONS.iter().cloned().map(|(k, v)| (k.to_string(), v.to_string())).collect()\n",
    );
    output.push_str("}\n");

    std::fs::write(&dest_path, output).unwrap();
}
