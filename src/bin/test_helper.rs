// A minimal helper binary used only by integration tests so that tests
// do not rely on Unix-specific shell commands (sh, touch, true, false).
//
// Usage:
//   test_helper                    — exits 0  (replaces `true`)
//   test_helper --fail             — exits 1  (replaces `false`)
//   test_helper --touch <path>     — creates the file at <path>, then exits 0
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--fail") {
        std::process::exit(1);
    }

    if let Some(i) = args.iter().position(|a| a == "--touch") {
        if let Some(path) = args.get(i + 1) {
            std::fs::write(path, b"").expect("test_helper: failed to create file");
        }
    }
}
