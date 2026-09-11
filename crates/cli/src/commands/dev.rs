use crate::{cli::DevArgs, commands::common::build_once};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::{path::PathBuf, time::Duration};
use tine_transpiler::{self, TranspileOptions};

pub fn run(args: DevArgs) {
    let path_buf = PathBuf::from(&args.input).canonicalize().unwrap();
    let output = args.output.clone().unwrap_or("out.js".into());

    build_once(&path_buf, &output, TranspileOptions::default());
    watch_and_rebuild(&path_buf, &output);
}

fn watch_and_rebuild(path_buf: &PathBuf, output: &str) {
    let root = if path_buf.is_dir() {
        path_buf.to_path_buf()
    } else {
        path_buf.parent().unwrap().to_path_buf()
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(150), tx).unwrap();

    debouncer
        .watcher()
        .watch(&root, RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    println!("Watching {} for changes...", root.display());

    for res in rx {
        match res {
            Ok(events) => {
                let relevant = events
                    .iter()
                    .any(|e| e.path.extension().and_then(|s| s.to_str()) == Some("tine"));
                if relevant {
                    println!("Change detected, rebuilding...");
                    build_once(path_buf, output, TranspileOptions::default());
                }
            }
            Err(e) => eprintln!("Watch error: {e:?}"),
        }
    }
}
